#![forbid(unsafe_code)]
#![doc = "Scheduler collectors: auto-discover systemd timer state, cron, at, anacron spool directories."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

/// Job scheduler directories to auto-discover.
const SCHEDULER_ROOTS: &[(&str, &str)] = &[
    ("timers", "/run/systemd/timers"),
    ("cron", "/var/spool/cron/crontabs"),
    ("at", "/var/spool/at"),
    ("anacron", "/var/spool/anacron"),
    ("fcron", "/var/spool/fcron"),
];

pub(crate) fn collect_scheduler(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let traversal_limits = BoundedTraversalLimits::new(64, 1, 3000);
    let read_limits = BoundedReadLimits::new(256 * 1024, 3000);

    for &(domain, root_path) in SCHEDULER_ROOTS {
        let root = Path::new(root_path);
        if !root.exists() {
            continue;
        }
        let listing = match list_dir_entries(root, traversal_limits) {
            Ok(l) => l,
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                store.record_object(
                    ManifestEntry::new(
                        format!("scheduler.{domain}.status"),
                        SourceSlug::Scheduler,
                        domain,
                        "status",
                        ObjectKind::Metadata,
                        status,
                    )
                    .with_reason(error.to_string())
                    .with_limits(ObjectLimits::new(
                        0,
                        traversal_limits.timeout_ms,
                        1,
                        0,
                    )),
                )?;
                continue;
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
                    store.record_object(
                        ManifestEntry::new(
                            format!("scheduler.{domain}.{name}", name = entry.name),
                            SourceSlug::Scheduler,
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
                        )),
                    )?;
                    total_captured += 1;
                }
                Err(error) => {
                    let status = fs_error_to_manifest_status(&error);
                    store.record_object(
                        ManifestEntry::new(
                            format!("scheduler.{domain}.{name}", name = entry.name),
                            SourceSlug::Scheduler,
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
                        )),
                    )?;
                }
            }
        }
        // Recording listing
        store.record_object(
            ManifestEntry::new(
                format!("scheduler.{domain}.listing"),
                SourceSlug::Scheduler,
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
            )),
        )?;
    }
    Ok(())
}
