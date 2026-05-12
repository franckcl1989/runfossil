#![forbid(unsafe_code)]
#![doc = "Container runtime detection and /run state collection."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{
    BoundedReadLimits, BoundedTraversalLimits, DirEntry, ListResult, list_dir_entries,
    read_file_bounded,
};
use runfossil_store::{ErrorLogEntry, ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

const DOCKER_DIR: &str = "/run/docker";
const CONTAINERD_DIR: &str = "/run/containerd";
const CRIO_DIR: &str = "/run/crio";
const RUNC_DIR: &str = "/run/runc";

const MAX_FILES: u64 = 64;
const MAX_DEPTH: u32 = 4;
const MAX_BYTES_PER_FILE: u64 = 262_144;
const TIMEOUT_MS: u64 = 5_000;

pub(crate) fn collect_run_state(store: &mut SnapshotStore) -> Result<(), StoreError> {
    for (dir, name) in &[
        (DOCKER_DIR, "docker"),
        (CONTAINERD_DIR, "containerd"),
        (CRIO_DIR, "crio"),
        (RUNC_DIR, "runc"),
    ] {
        collect_runtime_dir(store, Path::new(dir), name)?;
    }
    Ok(())
}

fn collect_runtime_dir(
    store: &mut SnapshotStore,
    source_path: &Path,
    runtime_name: &str,
) -> Result<(), StoreError> {
    if !source_path.is_dir() {
        return Ok(());
    }

    let limits = BoundedTraversalLimits::new(MAX_FILES, MAX_DEPTH, TIMEOUT_MS);
    let file_limits = BoundedReadLimits::new(MAX_BYTES_PER_FILE, TIMEOUT_MS);

    let listing = match list_dir_entries(source_path, limits) {
        Ok(listing) => listing,
        Err(error) => {
            let entry = ManifestEntry::new(
                format!("container.{runtime_name}.run_listing"),
                SourceSlug::Container,
                runtime_name,
                "run_listing",
                ObjectKind::DirListing,
                ManifestStatus::IoError,
            )
            .with_reason(format!("cannot list {source_path:?}: {error}"))
            .with_limits(ObjectLimits::new(0, TIMEOUT_MS, MAX_FILES, MAX_DEPTH));

            store.record_object(entry)?;
            return Ok(());
        }
    };

    record_run_listing(store, &listing, runtime_name)?;

    for entry in &listing.entries {
        let entry_path = source_path.join(&entry.name);
        capture_run_entry(store, &entry_path, runtime_name, entry, file_limits, limits)?;
    }

    Ok(())
}

fn record_run_listing(
    store: &mut SnapshotStore,
    listing: &ListResult,
    runtime_name: &str,
) -> Result<(), StoreError> {
    let status = if listing.was_truncated {
        ManifestStatus::Truncated
    } else {
        ManifestStatus::Captured
    };

    let entry = ManifestEntry::new(
        format!("container.{runtime_name}.listing"),
        SourceSlug::Container,
        runtime_name,
        "listing",
        ObjectKind::DirListing,
        status,
    )
    .with_bytes(listing.entries.len() as u64)
    .with_limits(ObjectLimits::new(0, TIMEOUT_MS, MAX_FILES, MAX_DEPTH));

    store.record_object(entry)?;
    Ok(())
}

fn capture_run_entry(
    store: &mut SnapshotStore,
    path: &Path,
    runtime_name: &str,
    dir_entry: &DirEntry,
    file_limits: BoundedReadLimits,
    dir_limits: BoundedTraversalLimits,
) -> Result<(), StoreError> {
    let name = &dir_entry.name;
    let out_base = format!("container/{runtime_name}");

    if dir_entry.is_dir {
        let sub_listing = match list_dir_entries(path, dir_limits) {
            Ok(listing) => listing,
            Err(error) => {
                let entry = ManifestEntry::new(
                    format!("container.{runtime_name}.{name}.listing"),
                    SourceSlug::Container,
                    runtime_name,
                    name,
                    ObjectKind::DirListing,
                    ManifestStatus::IoError,
                )
                .with_reason(format!("cannot list {path:?}: {error}"))
                .with_limits(ObjectLimits::new(0, TIMEOUT_MS, MAX_FILES, MAX_DEPTH));

                store.record_object(entry)?;
                return Ok(());
            }
        };

        let truncated = sub_listing.was_truncated;
        let status = if truncated {
            ManifestStatus::Truncated
        } else {
            ManifestStatus::Captured
        };

        let entry = ManifestEntry::new(
            format!("container.{runtime_name}.{name}.listing"),
            SourceSlug::Container,
            runtime_name,
            name,
            ObjectKind::DirListing,
            status,
        )
        .with_path(format!("{out_base}/{name}"))
        .with_bytes(sub_listing.entries.len() as u64)
        .with_limits(ObjectLimits::new(0, TIMEOUT_MS, MAX_FILES, MAX_DEPTH));

        store.record_object(entry)?;

        for sub in &sub_listing.entries {
            let sub_path = path.join(&sub.name);
            let sub_out = format!("{out_base}/{name}");

            if sub.is_dir {
                record_dir_metadata(store, &sub_path, runtime_name, name, sub)?;
            } else {
                capture_run_file(
                    store,
                    &sub_path,
                    &sub_out,
                    runtime_name,
                    &sub.name,
                    file_limits,
                )?;
            }
        }
        return Ok(());
    }

    capture_run_file(store, path, &out_base, runtime_name, name, file_limits)?;
    Ok(())
}

fn capture_run_file(
    store: &mut SnapshotStore,
    path: &Path,
    out_base: &str,
    runtime_name: &str,
    name: &str,
    limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    let out_path = format!("{out_base}/{name}");

    match read_file_bounded(path, limits) {
        Ok(result) => {
            let bytes = result.content.len() as u64;
            store.write_raw_file(Path::new(&out_path), &result.content)?;

            let status = if result.was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let entry = ManifestEntry::new(
                format!("container.{runtime_name}.{name}"),
                SourceSlug::Container,
                runtime_name,
                name,
                ObjectKind::File,
                status,
            )
            .with_path(&out_path)
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                format!("container.{runtime_name}.{name}"),
                SourceSlug::Container,
                runtime_name,
                name,
                ObjectKind::File,
                ManifestStatus::IoError,
            )
            .with_path(&out_path)
            .with_reason(format!("cannot read {path:?}: {error}"))
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn record_dir_metadata(
    store: &mut SnapshotStore,
    path: &Path,
    runtime_name: &str,
    parent: &str,
    sub: &DirEntry,
) -> Result<(), StoreError> {
    let out_rel = format!("container/{runtime_name}/{parent}/{}/", sub.name);
    let limits = BoundedTraversalLimits::new(MAX_FILES, MAX_DEPTH, TIMEOUT_MS);

    match list_dir_entries(path, limits) {
        Ok(listing) => {
            let entry = ManifestEntry::new(
                format!("container.{runtime_name}.{parent}.{}.listing", sub.name),
                SourceSlug::Container,
                runtime_name,
                &sub.name,
                ObjectKind::DirListing,
                ManifestStatus::Captured,
            )
            .with_path(&out_rel)
            .with_bytes(listing.entries.len() as u64)
            .with_limits(ObjectLimits::new(0, TIMEOUT_MS, MAX_FILES, MAX_DEPTH));

            store.record_object(entry)?;
        }
        Err(error) => {
            let error_log = ErrorLogEntry::new(
                "0",
                None::<String>,
                ManifestStatus::IoError,
                format!("cannot list container dir {path:?}: {error}"),
            );
            store.log_error(&error_log)?;
        }
    }

    Ok(())
}
