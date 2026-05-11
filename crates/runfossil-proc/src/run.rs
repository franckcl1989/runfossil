#![forbid(unsafe_code)]
#![doc = "/run runtime state collectors. Linux host state, not container-specific."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

pub(crate) fn collect_run(store: &mut SnapshotStore) -> Result<(), StoreError> {
    collect_run_systemd(store)?;
    collect_run_user(store)?;
    collect_run_lock(store)?;
    collect_run_pid_files(store)?;
    collect_run_udev(store)?;
    collect_run_dbus(store)?;
    Ok(())
}

fn collect_run_systemd(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/run/systemd");

    if !root.exists() {
        let entry = ManifestEntry::new(
            "run.systemd.status",
            SourceSlug::Run,
            "systemd",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("/run/systemd not present on this host")
        .with_limits(ObjectLimits::new(0, 5000, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    collect_bounded_tree(
        store,
        root,
        "systemd",
        BoundedTraversalLimits::new(64, 3, 5000),
        BoundedReadLimits::new(256 * 1024, 5000),
    )
}

fn collect_run_user(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/run/user");

    if !root.exists() {
        let entry = ManifestEntry::new(
            "run.user.status",
            SourceSlug::Run,
            "user",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("/run/user not present on this host")
        .with_limits(ObjectLimits::new(0, 3000, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    collect_bounded_tree(
        store,
        root,
        "user",
        BoundedTraversalLimits::new(32, 2, 3000),
        BoundedReadLimits::new(64 * 1024, 3000),
    )
}

fn collect_run_lock(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/run/lock");

    if !root.exists() {
        return Ok(());
    }

    collect_bounded_tree(
        store,
        root,
        "lock",
        BoundedTraversalLimits::new(32, 2, 3000),
        BoundedReadLimits::new(64 * 1024, 3000),
    )
}

fn collect_run_pid_files(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/run");

    let limits = BoundedTraversalLimits::new(64, 1, 3000);
    let read_limits = BoundedReadLimits::new(4 * 1024, 1000);

    let listing = match list_dir_entries(root, limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                "run.pid_files.status",
                SourceSlug::Run,
                "pid_files",
                "status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
            return Ok(());
        }
    };

    let mut found_any = false;

    for entry in &listing.entries {
        if entry.is_dir {
            continue;
        }
        if !entry.name.ends_with(".pid") {
            continue;
        }

        found_any = true;
        let source_path = root.join(&entry.name);

        match read_file_bounded(&source_path, read_limits) {
            Ok(result) => {
                let out_path = Path::new("raw/run").join(&entry.name);
                let bytes = result.content.len() as u64;

                store.write_raw_file(&out_path, &result.content)?;

                let status = if result.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };

                let file_entry = ManifestEntry::new(
                    format!(
                        "run.pid_files.{name}",
                        name = entry.name.trim_end_matches(".pid")
                    ),
                    SourceSlug::Run,
                    "pid_files",
                    &entry.name,
                    ObjectKind::File,
                    status,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(
                    read_limits.max_bytes,
                    read_limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(file_entry)?;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                let file_entry = ManifestEntry::new(
                    format!(
                        "run.pid_files.{name}",
                        name = entry.name.trim_end_matches(".pid")
                    ),
                    SourceSlug::Run,
                    "pid_files",
                    &entry.name,
                    ObjectKind::File,
                    status,
                )
                .with_reason(error.to_string())
                .with_limits(ObjectLimits::new(
                    read_limits.max_bytes,
                    read_limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(file_entry)?;
            }
        }
    }

    if !found_any {
        let manifest_entry = ManifestEntry::new(
            "run.pid_files.status",
            SourceSlug::Run,
            "pid_files",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("no *.pid files found in /run")
        .with_limits(ObjectLimits::new(0, 1000, 1, 0));

        store.record_object(manifest_entry)?;
    } else {
        let set_entry = ManifestEntry::new(
            "run.pid_files.set",
            SourceSlug::Run,
            "pid_files",
            "set",
            ObjectKind::FileSet,
            ManifestStatus::Captured,
        )
        .with_path("raw/run")
        .with_limits(ObjectLimits::new(
            read_limits.max_bytes,
            read_limits.timeout_ms,
            listing.entries.len() as u64,
            0,
        ));

        store.record_object(set_entry)?;
    }

    Ok(())
}

fn collect_run_udev(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/run/udev");

    if !root.exists() {
        return Ok(());
    }

    collect_bounded_tree(
        store,
        root,
        "udev",
        BoundedTraversalLimits::new(32, 2, 3000),
        BoundedReadLimits::new(64 * 1024, 3000),
    )
}

fn collect_run_dbus(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/run/dbus");

    if !root.exists() {
        return Ok(());
    }

    collect_bounded_tree(
        store,
        root,
        "dbus",
        BoundedTraversalLimits::new(32, 2, 3000),
        BoundedReadLimits::new(64 * 1024, 3000),
    )
}

fn collect_bounded_tree(
    store: &mut SnapshotStore,
    root: &Path,
    domain: &str,
    traversal_limits: BoundedTraversalLimits,
    read_limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    let listing = match list_dir_entries(root, traversal_limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("run.{domain}.status"),
                SourceSlug::Run,
                domain,
                "status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, traversal_limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
            return Ok(());
        }
    };

    let mut total_captured: u64 = 0;

    for entry in &listing.entries {
        if entry.is_dir {
            continue;
        }

        let source_path = root.join(&entry.name);
        match read_file_bounded(&source_path, read_limits) {
            Ok(result) => {
                let out_path = output_path(&source_path);
                let bytes = result.content.len() as u64;

                store.write_raw_file(&out_path, &result.content)?;

                let status = if result.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };

                let manifest_entry = ManifestEntry::new(
                    format!("run.{domain}.{name}", name = entry.name),
                    SourceSlug::Run,
                    domain,
                    &entry.name,
                    ObjectKind::File,
                    status,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(
                    read_limits.max_bytes,
                    read_limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(manifest_entry)?;
                total_captured += 1;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                let manifest_entry = ManifestEntry::new(
                    format!("run.{domain}.{name}", name = entry.name),
                    SourceSlug::Run,
                    domain,
                    &entry.name,
                    ObjectKind::File,
                    status,
                )
                .with_reason(error.to_string())
                .with_limits(ObjectLimits::new(
                    read_limits.max_bytes,
                    read_limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(manifest_entry)?;
            }
        }
    }

    let dir_entry = ManifestEntry::new(
        format!("run.{domain}.listing"),
        SourceSlug::Run,
        domain,
        "listing",
        ObjectKind::DirListing,
        ManifestStatus::Captured,
    )
    .with_bytes(total_captured)
    .with_limits(ObjectLimits::new(
        0,
        traversal_limits.timeout_ms,
        listing.entries.len() as u64,
        traversal_limits.max_depth,
    ));

    store.record_object(dir_entry)?;

    Ok(())
}
