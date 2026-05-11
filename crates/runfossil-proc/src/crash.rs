#![forbid(unsafe_code)]
#![doc = "Crash dump collectors: kdump, core_pattern, and coredump directory."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

pub(crate) fn collect_crash(store: &mut SnapshotStore) -> Result<(), StoreError> {
    collect_kdump(store)?;
    collect_core_pattern(store)?;
    collect_coredump(store)?;
    Ok(())
}

fn collect_kdump(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let paths: &[(&str, &str)] = &[
        ("crash.kdump.loaded", "/sys/kernel/kexec_crash_loaded"),
        ("crash.kdump.size", "/sys/kernel/kexec_crash_size"),
    ];

    let limits = BoundedReadLimits::new(4096, 100);

    for (id, path_str) in paths {
        let path = Path::new(path_str);

        if !path.exists() {
            let entry = ManifestEntry::new(
                *id,
                SourceSlug::Crash,
                "kdump",
                &path_str[1..],
                ObjectKind::Metadata,
                ManifestStatus::NotFound,
            )
            .with_reason(format!("{path_str} not present on this host"))
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
                    *id,
                    SourceSlug::Crash,
                    "kdump",
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
                    *id,
                    SourceSlug::Crash,
                    "kdump",
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

    Ok(())
}

fn collect_core_pattern(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let path = Path::new("/proc/sys/kernel/core_pattern");

    if !path.exists() {
        let entry = ManifestEntry::new(
            "crash.core.pattern",
            SourceSlug::Crash,
            "core",
            "kernel/core_pattern",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("/proc/sys/kernel/core_pattern not present on this host")
        .with_limits(ObjectLimits::new(0, 100, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let limits = BoundedReadLimits::new(256, 100);

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
                "crash.core.pattern",
                SourceSlug::Crash,
                "core",
                "kernel/core_pattern",
                ObjectKind::Metadata,
                status,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                "crash.core.pattern",
                SourceSlug::Crash,
                "core",
                "kernel/core_pattern",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn collect_coredump(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/var/lib/systemd/coredump");

    if !root.exists() {
        return Ok(());
    }

    let traversal_limits = BoundedTraversalLimits::new(32, 1, 5000);
    let read_limits = BoundedReadLimits::new(1024 * 1024, 5000);

    let listing = match list_dir_entries(root, traversal_limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                "crash.coredump.status",
                SourceSlug::Crash,
                "coredump",
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
                    format!("crash.coredump.{name}", name = entry.name),
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
                ));

                store.record_object(file_entry)?;
                total_captured += 1;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                let file_entry = ManifestEntry::new(
                    format!("crash.coredump.{name}", name = entry.name),
                    SourceSlug::Crash,
                    "coredump",
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
        "crash.coredump.listing",
        SourceSlug::Crash,
        "coredump",
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
