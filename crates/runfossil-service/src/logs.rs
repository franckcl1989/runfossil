#![forbid(unsafe_code)]
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
pub(crate) fn collect_log_windows(store: &SnapshotStore) -> Result<(), StoreError> {
    collect_journal_dir(store, Path::new("/run/log/journal"), "journal.run")?;
    collect_journal_dir(store, Path::new("/var/log/journal"), "journal.var")?;
    collect_standard_log_file(store, "syslog", Path::new("/var/log/syslog"))?;
    collect_standard_log_file(store, "messages", Path::new("/var/log/messages"))?;
    collect_standard_log_file(store, "kern.log", Path::new("/var/log/kern.log"))?;
    collect_standard_log_file(store, "auth.log", Path::new("/var/log/auth.log"))?;
    collect_standard_log_file(store, "dmesg", Path::new("/var/log/dmesg"))?;
    collect_standard_log_file(store, "lastlog", Path::new("/var/log/lastlog"))?;
    Ok(())
}

fn collect_journal_dir(store: &SnapshotStore, dir: &Path, domain: &str) -> Result<(), StoreError> {
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
                    let machine_dir = dir.join(&entry.name);
                    let sub_limits = BoundedTraversalLimits::new(MAX_LIST_FILES, 0, 100);
                    match list_dir_entries(&machine_dir, sub_limits) {
                        Ok(sub_listing) => {
                            for sub_entry in &sub_listing.entries {
                                if sub_entry.is_dir {
                                    continue;
                                }
                                let source_path = machine_dir.join(&sub_entry.name);
                                let out_path = Path::new("raw/logs")
                                    .join(domain)
                                    .join(&entry.name)
                                    .join(&sub_entry.name);
                                collect_log_file(
                                    store,
                                    domain,
                                    &format!("{}/{}", entry.name, sub_entry.name),
                                    &source_path,
                                    &out_path,
                                    read_limits,
                                )?;
                                captured += 1;
                            }
                        }
                        Err(error) => {
                            store.record_object(
                                ManifestEntry::new(
                                    format!("logs.{domain}.{}.status", id_component(&entry.name)),
                                    SourceSlug::Logs,
                                    domain,
                                    format!("{}/status", entry.name),
                                    ObjectKind::Metadata,
                                    ManifestStatus::IoError,
                                )
                                .with_reason(format!("failed to list {machine_dir:?}: {error}"))
                                .with_limits(ObjectLimits::new(0, 100, MAX_LIST_FILES, 0)),
                            )?;
                        }
                    }
                    continue;
                }

                let source_path = dir.join(&entry.name);
                let out_path = Path::new("raw/logs").join(domain).join(&entry.name);
                collect_log_file(
                    store,
                    domain,
                    &entry.name,
                    &source_path,
                    &out_path,
                    read_limits,
                )?;
                captured += 1;
            }

            if captured == 0 {
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

fn collect_standard_log_file(
    store: &SnapshotStore,
    domain: &str,
    source: &Path,
) -> Result<(), StoreError> {
    if !source.exists() {
        let entry = ManifestEntry::new(
            format!("logs.{}.status", id_component(domain)),
            SourceSlug::Logs,
            domain,
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason(format!("{} not present", source.display()))
        .with_limits(ObjectLimits::new(0, 50, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let limits = BoundedReadLimits::new(MAX_FILE_BYTES, 500);
    let out_path = Path::new("raw/logs").join(format!("{domain}.window"));
    collect_log_file(store, domain, "window", source, &out_path, limits)
}

fn collect_log_file(
    store: &SnapshotStore,
    domain: &str,
    object: &str,
    source: &Path,
    out_path: &Path,
    limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    match read_file_bounded(source, limits) {
        Ok(result) => {
            let bytes = result.content.len() as u64;
            store.write_raw_file(out_path, &result.content)?;

            let status = if result.was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let entry = ManifestEntry::new(
                log_window_id(domain, object),
                SourceSlug::Logs,
                domain,
                object,
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
                log_window_id(domain, object),
                SourceSlug::Logs,
                domain,
                object,
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

fn log_window_id(domain: &str, object: &str) -> String {
    if object == "window" {
        format!("logs.{}.window", id_component(domain))
    } else {
        format!(
            "logs.{}.{}.window",
            id_component(domain),
            id_component(object)
        )
    }
}

fn id_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' => output.push(byte as char),
            b'A'..=b'Z' => output.push((byte + 32) as char),
            b'.' => {
                if !output.is_empty() && !output.ends_with('.') {
                    output.push('.');
                }
            }
            _ => {
                if !output.is_empty() && !output.ends_with('.') {
                    output.push('.');
                }
            }
        }
    }
    if output.ends_with('.') {
        output.pop();
    }
    if output.is_empty() {
        "object".to_string()
    } else {
        output
    }
}
