#![forbid(unsafe_code)]
#![doc = "Crash dump collectors: kdump, core_pattern, and coredump directory. Auto-discover available paths."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

/// Crash evidence source paths. Check existence, then collect.
const CRASH_PATHS: &[(&str, &str, &str)] = &[
    // (id_prefix, source_path, domain)
    (
        "crash.kdump.loaded",
        "/sys/kernel/kexec_crash_loaded",
        "kdump",
    ),
    ("crash.kdump.size", "/sys/kernel/kexec_crash_size", "kdump"),
    (
        "crash.core.pattern",
        "/proc/sys/kernel/core_pattern",
        "core",
    ),
];

pub(crate) fn collect_crash(store: &mut SnapshotStore) -> Result<(), StoreError> {
    // Collect known crash evidence files (auto-discovered by existence check)
    let limits = BoundedReadLimits::new(4096, 100);
    for &(id, path_str, domain) in CRASH_PATHS {
        let path = Path::new(path_str);
        if !path.exists() {
            let entry = ManifestEntry::new(
                id,
                SourceSlug::Crash,
                domain,
                &path_str[1..],
                ObjectKind::Metadata,
                ManifestStatus::NotFound,
            )
            .with_reason(format!("{path_str} not present"))
            .with_limits(ObjectLimits::new(0, 100, 1, 0));
            store.record_object(entry)?;
            continue;
        }
        match read_file_bounded(path, limits) {
            Ok(result) => {
                let out_path = output_path(path);
                let bytes = result.content.len() as u64;
                store.write_raw_file(&out_path, &result.content)?;
                let status = if result.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };
                let entry = ManifestEntry::new(
                    id,
                    SourceSlug::Crash,
                    domain,
                    &path_str[1..],
                    ObjectKind::Metadata,
                    status,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(
                    limits.max_bytes,
                    limits.timeout_ms,
                    1,
                    0,
                ));
                store.record_object(entry)?;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                let entry = ManifestEntry::new(
                    id,
                    SourceSlug::Crash,
                    domain,
                    &path_str[1..],
                    ObjectKind::Metadata,
                    status,
                )
                .with_reason(error.to_string())
                .with_limits(ObjectLimits::new(
                    limits.max_bytes,
                    limits.timeout_ms,
                    1,
                    0,
                ));
                store.record_object(entry)?;
            }
        }
    }

    // Auto-discover coredump directory
    let coredump_dirs = &["/var/lib/systemd/coredump", "/var/crash"];
    for dir_path in coredump_dirs {
        let root = Path::new(dir_path);
        if !root.exists() {
            continue;
        }
        let traversal_limits = BoundedTraversalLimits::new(32, 1, 5000);
        let read_limits = BoundedReadLimits::new(1024 * 1024, 5000);
        if let Ok(listing) = list_dir_entries(root, traversal_limits) {
            for entry in &listing.entries {
                if entry.is_dir {
                    continue;
                }
                let source_path = root.join(&entry.name);
                if let Ok(result) = read_file_bounded(&source_path, read_limits) {
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
                            format!(
                                "crash.{}.{}",
                                root.file_name().unwrap_or_default().to_string_lossy(),
                                entry.name
                            ),
                            SourceSlug::Crash,
                            "coredump",
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
                }
            }
        }
    }

    Ok(())
}
