#![forbid(unsafe_code)]
#![doc = "Security subsystem collectors: SELinux, AppArmor, and LSM info."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

pub(crate) fn collect_security(store: &mut SnapshotStore) -> Result<(), StoreError> {
    collect_selinux(store)?;
    collect_apparmor(store)?;
    collect_lsm_info(store)?;
    Ok(())
}

fn collect_selinux(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/fs/selinux");

    if !root.exists() {
        let entry = ManifestEntry::new(
            "security.selinux.status",
            SourceSlug::Security,
            "selinux",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("/sys/fs/selinux not present on this host")
        .with_limits(ObjectLimits::new(0, 5000, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let traversal_limits = BoundedTraversalLimits::new(64, 2, 5000);
    let read_limits = BoundedReadLimits::new(256 * 1024, 5000);

    walk_bounded_tree(
        store,
        root,
        root,
        SourceSlug::Security,
        "selinux",
        traversal_limits,
        read_limits,
        0,
    )
}

fn collect_apparmor(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/kernel/security/apparmor");

    if !root.exists() {
        return Ok(());
    }

    let traversal_limits = BoundedTraversalLimits::new(64, 2, 5000);
    let read_limits = BoundedReadLimits::new(256 * 1024, 5000);

    walk_bounded_tree(
        store,
        root,
        root,
        SourceSlug::Security,
        "apparmor",
        traversal_limits,
        read_limits,
        0,
    )
}

fn collect_lsm_info(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let path = Path::new("/sys/kernel/security/lsm");

    if !path.exists() {
        return Ok(());
    }

    let limits = BoundedReadLimits::new(4096, 100);

    match read_file_bounded(path, limits) {
        Ok(result) => {
            let out_path = Path::new("raw/security/lsm");
            let bytes = result.content.len() as u64;
            store.write_raw_file(out_path, &result.content)?;

            let status = if result.was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let entry = ManifestEntry::new(
                "security.lsm",
                SourceSlug::Security,
                "lsm",
                "lsm",
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
                "security.lsm",
                SourceSlug::Security,
                "lsm",
                "lsm",
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

#[allow(clippy::too_many_arguments)]
fn walk_bounded_tree(
    store: &mut SnapshotStore,
    base: &Path,
    root: &Path,
    source: SourceSlug,
    domain: &str,
    traversal_limits: BoundedTraversalLimits,
    read_limits: BoundedReadLimits,
    depth: u32,
) -> Result<(), StoreError> {
    if depth >= traversal_limits.max_depth {
        return Ok(());
    }

    let listing = match list_dir_entries(root, traversal_limits) {
        Ok(listing) => listing,
        Err(error) => {
            if depth == 0 {
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
                .with_limits(ObjectLimits::new(
                    0,
                    traversal_limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(entry)?;
            }
            return Ok(());
        }
    };

    let mut total_captured: u64 = 0;

    for entry in &listing.entries {
        let source_path = root.join(&entry.name);

        if entry.is_dir {
            walk_bounded_tree(
                store,
                base,
                &source_path,
                source,
                domain,
                traversal_limits,
                read_limits,
                depth + 1,
            )?;
        } else {
            let rel = source_path
                .strip_prefix(base)
                .unwrap_or(&source_path)
                .to_string_lossy()
                .replace('/', ".");
            let object = source_path
                .strip_prefix("/")
                .unwrap_or(&source_path)
                .to_string_lossy()
                .into_owned();

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
                        format!("{}.{domain}.{rel}", source.as_str()),
                        source,
                        domain,
                        object,
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
                        format!("{}.{domain}.{rel}", source.as_str()),
                        source,
                        domain,
                        object,
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
    }

    if depth == 0 {
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
    }

    Ok(())
}
