#![forbid(unsafe_code)]
#![doc = "Scheduler collectors: systemd timers, cron, and at job files."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

pub(crate) fn collect_scheduler(store: &mut SnapshotStore) -> Result<(), StoreError> {
    collect_systemd_timers(store)?;
    collect_cron(store)?;
    collect_at_jobs(store)?;
    Ok(())
}

fn collect_systemd_timers(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/run/systemd/timers");

    if !root.exists() {
        return Ok(());
    }

    let traversal_limits = BoundedTraversalLimits::new(32, 1, 3000);
    let read_limits = BoundedReadLimits::new(256 * 1024, 3000);

    collect_bounded_tree(
        store,
        root,
        SourceSlug::Scheduler,
        "timers",
        traversal_limits,
        read_limits,
    )
}

fn collect_cron(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/var/spool/cron/crontabs");

    if !root.exists() {
        return Ok(());
    }

    let traversal_limits = BoundedTraversalLimits::new(64, 1, 3000);
    let read_limits = BoundedReadLimits::new(64 * 1024, 3000);

    collect_bounded_tree(
        store,
        root,
        SourceSlug::Scheduler,
        "cron",
        traversal_limits,
        read_limits,
    )
}

fn collect_at_jobs(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/var/spool/at");

    if !root.exists() {
        return Ok(());
    }

    let traversal_limits = BoundedTraversalLimits::new(64, 1, 3000);
    let read_limits = BoundedReadLimits::new(64 * 1024, 3000);

    collect_bounded_tree(
        store,
        root,
        SourceSlug::Scheduler,
        "at",
        traversal_limits,
        read_limits,
    )
}

fn collect_bounded_tree(
    store: &mut SnapshotStore,
    root: &Path,
    source: SourceSlug,
    domain: &str,
    traversal_limits: BoundedTraversalLimits,
    read_limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    let listing = match list_dir_entries(root, traversal_limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("{}.{domain}.status", source.as_str()),
                source,
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

                let file_entry = ManifestEntry::new(
                    format!("{}.{domain}.{name}", source.as_str(), name = entry.name),
                    source,
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

                store.record_object(file_entry)?;
                total_captured += 1;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                let file_entry = ManifestEntry::new(
                    format!("{}.{domain}.{name}", source.as_str(), name = entry.name),
                    source,
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

                store.record_object(file_entry)?;
            }
        }
    }

    let dir_entry = ManifestEntry::new(
        format!("{}.{domain}.listing", source.as_str()),
        source,
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
