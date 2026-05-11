#![doc = "/dev metadata and symlink-targets collector. Device nodes are metadata-only except for /dev/kmsg."]

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_link_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

fn dev_type_label(meta: &fs::Metadata) -> &'static str {
    let ft = meta.file_type();
    if ft.is_block_device() {
        "block"
    } else if ft.is_char_device() {
        "character"
    } else if ft.is_dir() {
        "directory"
    } else if ft.is_fifo() {
        "fifo"
    } else if ft.is_symlink() {
        "symlink"
    } else if ft.is_file() {
        "regular"
    } else if ft.is_socket() {
        "socket"
    } else {
        "unknown"
    }
}

fn metadata_line(path: &Path, meta: &fs::Metadata) -> String {
    let dev = meta.rdev();
    let major = (dev >> 8) & 0xfff;
    let minor = dev & 0xff | ((dev >> 12) & !0xfff);

    format!(
        "{}\tmode:{:o}\tuid:{}:gid:{}\tsize:{}\ttype:{}\tdev:{major}:{minor}\n",
        path.file_name().unwrap_or_default().to_string_lossy(),
        meta.mode() & 0o7777,
        meta.uid(),
        meta.gid(),
        meta.len(),
        dev_type_label(meta),
    )
}

fn record_dev_top_level(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let dev_dir = Path::new("/dev");
    let limits = BoundedTraversalLimits::new(2048, 1, 500);

    let listing = match list_dir_entries(dev_dir, limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                "dev.top.status",
                SourceSlug::Dev,
                "top",
                "status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, 500, 1, 0));

            store.record_object(entry)?;
            return Ok(());
        }
    };

    let mut metadata_content = String::new();

    for entry in &listing.entries {
        let source_path = dev_dir.join(&entry.name);
        match fs::symlink_metadata(&source_path) {
            Ok(meta) => {
                metadata_content.push_str(&metadata_line(&source_path, &meta));
            }
            Err(_) => {
                metadata_content.push_str(&format!("{}\tERROR\n", entry.name));
            }
        }
    }

    let out_path = Path::new("raw/dev/_listing");
    store.write_raw_file(out_path, metadata_content.as_bytes())?;

    let manifest_entry = ManifestEntry::new(
        "dev.top.listing",
        SourceSlug::Dev,
        "top",
        "listing",
        ObjectKind::DirListing,
        ManifestStatus::Captured,
    )
    .with_path(out_path.to_string_lossy())
    .with_bytes(metadata_content.len() as u64)
    .with_limits(ObjectLimits::new(
        0,
        limits.timeout_ms,
        listing.entries.len() as u64,
        limits.max_depth,
    ));

    store.record_object(manifest_entry)?;
    Ok(())
}

fn collect_dev_symlink_dir(
    store: &mut SnapshotStore,
    dir_path: &Path,
    domain: &str,
) -> Result<(), StoreError> {
    if !dir_path.exists() {
        return Ok(());
    }

    let limits = BoundedTraversalLimits::new(256, 1, 200);
    let read_limits = BoundedReadLimits::new(4096, 100);

    let listing = match list_dir_entries(dir_path, limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("dev.{domain}.status"),
                SourceSlug::Dev,
                domain,
                "status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, 200, 1, 0));

            store.record_object(entry)?;
            return Ok(());
        }
    };

    for entry in &listing.entries {
        let source_path = dir_path.join(&entry.name);
        match read_link_bounded(&source_path, read_limits) {
            Ok(target) => {
                let out_path = output_path(&source_path);
                let content = target.to_string_lossy().into_owned();
                store.write_raw_file(&out_path, content.as_bytes())?;

                let manifest_entry = ManifestEntry::new(
                    format!("dev.{domain}.{}", entry.name),
                    SourceSlug::Dev,
                    domain,
                    &entry.name,
                    ObjectKind::Symlink,
                    ManifestStatus::Captured,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(content.len() as u64)
                .with_limits(ObjectLimits::new(
                    read_limits.max_bytes,
                    read_limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(manifest_entry)?;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                let manifest_entry = ManifestEntry::new(
                    format!("dev.{domain}.{}", entry.name),
                    SourceSlug::Dev,
                    domain,
                    &entry.name,
                    ObjectKind::Symlink,
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

    Ok(())
}

fn collect_pseudo_devices(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let pseudo_devices: &[(&str, &str)] = &[
        ("null", "null"),
        ("zero", "zero"),
        ("full", "full"),
        ("random", "random"),
        ("urandom", "urandom"),
        ("console", "console"),
        ("tty-current", "tty"),
        ("fuse", "fuse"),
    ];

    for (object, name) in pseudo_devices {
        let source_path = Path::new("/dev").join(name);

        match fs::symlink_metadata(&source_path) {
            Ok(meta) => {
                let out_path = output_path(&source_path);
                let content = metadata_line(&source_path, &meta);
                store.write_raw_file(&out_path, content.as_bytes())?;

                let manifest_entry = ManifestEntry::new(
                    format!("dev.pseudo.{object}"),
                    SourceSlug::Dev,
                    "pseudo",
                    *object,
                    ObjectKind::Metadata,
                    ManifestStatus::Captured,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(content.len() as u64)
                .with_limits(ObjectLimits::new(256, 100, 1, 0));

                store.record_object(manifest_entry)?;
            }
            Err(error) => {
                let status = match error.kind() {
                    std::io::ErrorKind::NotFound => ManifestStatus::NotFound,
                    std::io::ErrorKind::PermissionDenied => ManifestStatus::PermissionDenied,
                    _ => ManifestStatus::IoError,
                };

                let manifest_entry = ManifestEntry::new(
                    format!("dev.pseudo.{object}"),
                    SourceSlug::Dev,
                    "pseudo",
                    *object,
                    ObjectKind::Metadata,
                    status,
                )
                .with_reason(error.to_string())
                .with_limits(ObjectLimits::new(256, 100, 1, 0));

                store.record_object(manifest_entry)?;
            }
        }
    }

    Ok(())
}

fn collect_loop_devices(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let dev_dir = Path::new("/dev");
    let limits = BoundedTraversalLimits::new(256, 1, 200);

    let listing = match list_dir_entries(dev_dir, limits) {
        Ok(listing) => listing,
        Err(_error) => return Ok(()),
    };

    let mut found_any = false;

    for entry in &listing.entries {
        if !entry.name.starts_with("loop") {
            continue;
        }

        found_any = true;
        let source_path = dev_dir.join(&entry.name);

        match fs::symlink_metadata(&source_path) {
            Ok(meta) => {
                let out_path = output_path(&source_path);
                let content = metadata_line(&source_path, &meta);
                store.write_raw_file(&out_path, content.as_bytes())?;

                let manifest_entry = ManifestEntry::new(
                    format!("dev.loop.{}", entry.name),
                    SourceSlug::Dev,
                    "loop",
                    &entry.name,
                    ObjectKind::Metadata,
                    ManifestStatus::Captured,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(content.len() as u64)
                .with_limits(ObjectLimits::new(256, 100, 1, 0));

                store.record_object(manifest_entry)?;
            }
            Err(error) => {
                let status = match error.kind() {
                    std::io::ErrorKind::NotFound => ManifestStatus::NotFound,
                    std::io::ErrorKind::PermissionDenied => ManifestStatus::PermissionDenied,
                    _ => ManifestStatus::IoError,
                };

                let manifest_entry = ManifestEntry::new(
                    format!("dev.loop.{}", entry.name),
                    SourceSlug::Dev,
                    "loop",
                    &entry.name,
                    ObjectKind::Metadata,
                    status,
                )
                .with_reason(error.to_string())
                .with_limits(ObjectLimits::new(256, 100, 1, 0));

                store.record_object(manifest_entry)?;
            }
        }
    }

    if !found_any {
        let manifest_entry = ManifestEntry::new(
            "dev.loop.status",
            SourceSlug::Dev,
            "loop",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("no loop devices found")
        .with_limits(ObjectLimits::new(0, 100, 1, 0));

        store.record_object(manifest_entry)?;
    }

    Ok(())
}

/// Collects all `/dev` metadata and symlink-target evidence.
pub(crate) fn collect_dev(store: &mut SnapshotStore) -> Result<(), StoreError> {
    record_dev_top_level(store)?;
    collect_dev_symlink_dir(store, Path::new("/dev/block"), "block")?;
    collect_dev_symlink_dir(store, Path::new("/dev/disk"), "disk")?;
    collect_dev_symlink_dir(store, Path::new("/dev/mapper"), "mapper")?;
    collect_pseudo_devices(store)?;
    collect_loop_devices(store)?;
    Ok(())
}
