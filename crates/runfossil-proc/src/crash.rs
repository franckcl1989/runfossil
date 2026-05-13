#![forbid(unsafe_code)]
#![doc = "Crash dump collectors: kdump, core_pattern, and coredump directory. Auto-discover available paths."]

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
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

pub(crate) fn collect_crash(store: &SnapshotStore) -> Result<(), StoreError> {
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

    // Auto-discover coredump and crash directories as metadata-only evidence.
    let coredump_dirs = &[
        ("coredump", "/var/lib/systemd/coredump"),
        ("crash_log", "/var/crash"),
    ];
    for &(domain, dir_path) in coredump_dirs {
        let root = Path::new(dir_path);
        if !root.exists() {
            store.record_object(
                ManifestEntry::new(
                    format!("crash.{domain}.status"),
                    SourceSlug::Crash,
                    domain,
                    dir_path,
                    ObjectKind::MetadataOnly,
                    ManifestStatus::NotFound,
                )
                .with_reason(format!("{dir_path} not present"))
                .with_limits(ObjectLimits::new(0, 100, 1, 0)),
            )?;
            continue;
        }
        let traversal_limits = BoundedTraversalLimits::new(32, 1, 5000);
        match list_dir_entries(root, traversal_limits) {
            Ok(listing) => {
                let mut metadata = String::new();
                for entry in &listing.entries {
                    let source_path = root.join(&entry.name);
                    match fs::symlink_metadata(&source_path) {
                        Ok(meta) => metadata.push_str(&metadata_line(&source_path, &meta)),
                        Err(error) => {
                            metadata.push_str(&format!(
                                "{}\tERROR:{}\n",
                                source_path.display(),
                                error
                            ));
                        }
                    }
                }

                let out_path = Path::new("raw/crash").join(format!("{domain}.listing"));
                store.write_raw_file(&out_path, metadata.as_bytes())?;
                let status = if listing.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };

                store.record_object(
                    ManifestEntry::new(
                        format!("crash.{domain}.listing"),
                        SourceSlug::Crash,
                        domain,
                        dir_path,
                        ObjectKind::DirListing,
                        status,
                    )
                    .with_path(out_path.to_string_lossy())
                    .with_bytes(metadata.len() as u64)
                    .with_limits(ObjectLimits::new(
                        0,
                        traversal_limits.timeout_ms,
                        listing.entries.len() as u64,
                        traversal_limits.max_depth,
                    )),
                )?;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                store.record_object(
                    ManifestEntry::new(
                        format!("crash.{domain}.status"),
                        SourceSlug::Crash,
                        domain,
                        dir_path,
                        ObjectKind::MetadataOnly,
                        status,
                    )
                    .with_reason(error.to_string())
                    .with_limits(ObjectLimits::new(
                        0,
                        traversal_limits.timeout_ms,
                        1,
                        traversal_limits.max_depth,
                    )),
                )?;
            }
        }
    }

    Ok(())
}

fn metadata_line(path: &Path, meta: &fs::Metadata) -> String {
    format!(
        "{}\tmode:{:o}\tuid:{}\tgid:{}\tsize:{}\ttype:{}\n",
        path.display(),
        meta.mode() & 0o7777,
        meta.uid(),
        meta.gid(),
        meta.len(),
        file_type_label(meta),
    )
}

fn file_type_label(meta: &fs::Metadata) -> &'static str {
    let file_type = meta.file_type();
    if file_type.is_file() {
        "regular"
    } else if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_socket() {
        "socket"
    } else if file_type.is_fifo() {
        "fifo"
    } else if file_type.is_block_device() {
        "block"
    } else if file_type.is_char_device() {
        "character"
    } else {
        "unknown"
    }
}
