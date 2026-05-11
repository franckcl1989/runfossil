#![doc = "Bounded system log and journal window collector."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{
    BoundedReadLimits, BoundedTraversalLimits, list_dir_entries, read_file_bounded,
};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

const MAX_FILE_BYTES: u64 = 262_144;
const MAX_LIST_FILES: u64 = 32;

/// Collects bounded journal and log file windows from standard locations.
pub(crate) fn collect_log_windows(store: &mut SnapshotStore) -> Result<(), StoreError> {
    collect_journal_dir(store, Path::new("/run/log/journal"), "journal.run")?;
    collect_journal_dir(store, Path::new("/var/log/journal"), "journal.var")?;
    collect_lastlog(store)?;
    Ok(())
}

fn collect_journal_dir(
    store: &mut SnapshotStore,
    dir: &Path,
    domain: &str,
) -> Result<(), StoreError> {
    if !dir.exists() {
        let entry = ManifestEntry::new(
            format!("logs.{domain}.status"),
            SourceSlug::Logs,
            domain,
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason(format!("{dir:?} not present"))
        .with_limits(ObjectLimits::new(0, 50, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let limits = BoundedTraversalLimits::new(MAX_LIST_FILES, 3, 100);
    match list_dir_entries(dir, limits) {
        Ok(listing) => {
            let read_limits = BoundedReadLimits::new(MAX_FILE_BYTES, 500);
            let mut captured = 0u64;

            for entry in &listing.entries {
                if entry.is_dir {
                    continue;
                }

                let source_path = dir.join(&entry.name);
                match read_file_bounded(&source_path, read_limits) {
                    Ok(result) => {
                        let out_path = Path::new("raw/logs").join(domain).join(&entry.name);
                        let bytes = result.content.len() as u64;
                        store.write_raw_file(&out_path, &result.content)?;

                        let status = if result.was_truncated {
                            ManifestStatus::Truncated
                        } else {
                            ManifestStatus::Captured
                        };

                        let entry = ManifestEntry::new(
                            format!("logs.{domain}.{}.window", entry.name),
                            SourceSlug::Logs,
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

                        store.record_object(entry)?;
                        captured += 1;
                    }
                    Err(error) => {
                        let entry = ManifestEntry::new(
                            format!("logs.{domain}.{}.window", entry.name),
                            SourceSlug::Logs,
                            domain,
                            &entry.name,
                            ObjectKind::File,
                            ManifestStatus::IoError,
                        )
                        .with_reason(error.to_string())
                        .with_limits(ObjectLimits::new(
                            read_limits.max_bytes,
                            read_limits.timeout_ms,
                            1,
                            0,
                        ));

                        store.record_object(entry)?;
                    }
                }
            }

            if captured == 0 && listing.entries.is_empty() {
                let entry = ManifestEntry::new(
                    format!("logs.{domain}.status"),
                    SourceSlug::Logs,
                    domain,
                    "status",
                    ObjectKind::Metadata,
                    ManifestStatus::Captured,
                )
                .with_reason("journal directory exists but no readable files found")
                .with_limits(ObjectLimits::new(0, 50, 1, 0));

                store.record_object(entry)?;
            }
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                format!("logs.{domain}.status"),
                SourceSlug::Logs,
                domain,
                "status",
                ObjectKind::Metadata,
                ManifestStatus::IoError,
            )
            .with_reason(format!("failed to list {dir:?}: {error}"))
            .with_limits(ObjectLimits::new(0, 50, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn collect_lastlog(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let source = Path::new("/var/log/lastlog");

    if !source.exists() {
        let entry = ManifestEntry::new(
            "logs.lastlog.status",
            SourceSlug::Logs,
            "lastlog",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("/var/log/lastlog not present")
        .with_limits(ObjectLimits::new(0, 50, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let limits = BoundedReadLimits::new(MAX_FILE_BYTES, 500);
    match read_file_bounded(source, limits) {
        Ok(result) => {
            let out_path = Path::new("raw/logs/lastlog.window");
            let bytes = result.content.len() as u64;
            store.write_raw_file(out_path, &result.content)?;

            let status = if result.was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let entry = ManifestEntry::new(
                "logs.lastlog.window",
                SourceSlug::Logs,
                "lastlog",
                "window",
                ObjectKind::File,
                status,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                "logs.lastlog.window",
                SourceSlug::Logs,
                "lastlog",
                "window",
                ObjectKind::File,
                ManifestStatus::IoError,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}
