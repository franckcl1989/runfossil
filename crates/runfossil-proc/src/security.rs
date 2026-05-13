#![forbid(unsafe_code)]
#![doc = "Security subsystem collectors: SELinux, AppArmor, LSM, IMA, EVM, lockdown. Auto-discover available paths."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

/// Security source directories to probe and auto-discover.
const SECURITY_ROOTS: &[(&str, &str)] = &[
    ("selinux", "/sys/fs/selinux"),
    ("apparmor", "/sys/kernel/security/apparmor"),
    ("ima", "/sys/kernel/security/ima"),
    ("evm", "/sys/kernel/security/evm"),
];

/// Security single files to read if present.
const SECURITY_FILES: &[(&str, &str, &str, &str)] = &[
    // (id, path, domain, object)
    ("security.lsm", "/sys/kernel/security/lsm", "lsm", "lsm"),
    (
        "security.lockdown",
        "/sys/kernel/security/lockdown",
        "lockdown",
        "lockdown",
    ),
];

pub(crate) fn collect_security(store: &SnapshotStore) -> Result<(), StoreError> {
    // Auto-discover security directory trees
    let traversal_limits = BoundedTraversalLimits::new(64, 2, 5000);
    let read_limits = BoundedReadLimits::new(256 * 1024, 5000);

    for &(domain, root_path) in SECURITY_ROOTS {
        let root = Path::new(root_path);
        if !root.exists() {
            let entry = ManifestEntry::new(
                format!("security.{domain}.status"),
                SourceSlug::Security,
                domain,
                "status",
                ObjectKind::Metadata,
                ManifestStatus::NotFound,
            )
            .with_reason(format!("{root_path} not present"))
            .with_limits(ObjectLimits::new(0, 5000, 1, 0));
            store.record_object(entry)?;
            continue;
        }
        walk_security_tree(store, root, root, domain, traversal_limits, read_limits, 0)?;
    }

    // Read specific security files
    let single_limits = BoundedReadLimits::new(4096, 100);
    for &(id, path_str, domain, object) in SECURITY_FILES {
        let path = Path::new(path_str);
        if !path.exists() {
            continue;
        }
        if let Ok(result) = read_file_bounded(path, single_limits) {
            let out_path = output_path(path);
            let out_str = out_path.to_string_lossy().into_owned();
            let bytes = result.content.len() as u64;
            store.write_raw_file(&out_path, &result.content)?;
            let entry = ManifestEntry::new(
                id,
                SourceSlug::Security,
                domain,
                object,
                ObjectKind::Metadata,
                ManifestStatus::Captured,
            )
            .with_path(out_str)
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(
                single_limits.max_bytes,
                single_limits.timeout_ms,
                1,
                0,
            ));
            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn walk_security_tree(
    store: &SnapshotStore,
    base: &Path,
    root: &Path,
    domain: &str,
    traversal_limits: BoundedTraversalLimits,
    read_limits: BoundedReadLimits,
    depth: u32,
) -> Result<(), StoreError> {
    if depth >= traversal_limits.max_depth {
        return Ok(());
    }
    let listing = match list_dir_entries(root, traversal_limits) {
        Ok(l) => l,
        Err(error) => {
            if depth == 0 {
                let status = fs_error_to_manifest_status(&error);
                store.record_object(
                    ManifestEntry::new(
                        format!("security.{domain}.status"),
                        SourceSlug::Security,
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
            }
            return Ok(());
        }
    };

    for entry in &listing.entries {
        let source_path = root.join(&entry.name);
        if entry.is_dir {
            walk_security_tree(
                store,
                base,
                &source_path,
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
                        format!("security.{domain}.{rel}"),
                        SourceSlug::Security,
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
            }
        }
    }
    Ok(())
}
