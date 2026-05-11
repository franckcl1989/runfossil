#![doc = "Sysfs collector: /sys/fs/cgroup detection, pstore capture, class device capture."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded, read_link_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

/// Detected cgroup version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CgroupVersion {
    /// Cgroup v1 (separate hierarchies per controller).
    V1,
    /// Cgroup v2 (unified hierarchy).
    V2,
    /// Both v1 and v2 present (hybrid).
    Hybrid,
    /// No cgroup filesystem found.
    None,
}

impl CgroupVersion {
    /// Returns the cgroup version as a string for capture metadata.
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V2 => "v2",
            Self::Hybrid => "hybrid",
            Self::None => "none",
        }
    }
}

/// Detects the active cgroup version on this host.
pub(crate) fn detect_cgroup_version() -> CgroupVersion {
    let cgroup_root = Path::new("/sys/fs/cgroup");
    if !cgroup_root.exists() {
        return CgroupVersion::None;
    }

    let has_v1 = cgroup_root.join("cpuset").is_dir()
        || cgroup_root.join("memory").is_dir()
        || cgroup_root.join("cpu").is_dir();

    let has_v2 = has_unified_cgroup_v2(cgroup_root);

    match (has_v1, has_v2) {
        (true, true) => CgroupVersion::Hybrid,
        (true, false) => CgroupVersion::V1,
        (false, true) => CgroupVersion::V2,
        (false, false) => CgroupVersion::None,
    }
}

fn has_unified_cgroup_v2(root: &Path) -> bool {
    let cgroup_controllers = root.join("cgroup.controllers");
    if cgroup_controllers.exists() {
        return true;
    }

    let default_controllers = root.join("cgroup.subtree_control");
    if default_controllers.exists() {
        return true;
    }

    false
}

/// Collects all /sys evidence into the given store.
pub(crate) fn collect_sys(store: &mut SnapshotStore) -> Result<(), StoreError> {
    record_cgroup_version(store)?;
    collect_pstore(store)?;
    collect_power(store)?;
    collect_class_net(store)?;
    collect_block(store)?;
    Ok(())
}

fn record_cgroup_version(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let version = detect_cgroup_version();
    let content = format!("cgroup_version: {}\n", version.as_str());

    let out_path = Path::new("raw/sys/fs/cgroup/__version");
    store.write_raw_file(out_path, content.as_bytes())?;

    let entry = ManifestEntry::new(
        "sys.cgroup.version",
        SourceSlug::Sys,
        "cgroup",
        "version",
        ObjectKind::Metadata,
        ManifestStatus::Captured,
    )
    .with_path(out_path.to_string_lossy())
    .with_bytes(content.len() as u64)
    .with_limits(ObjectLimits::new(256, 50, 1, 0));

    store.record_object(entry)?;
    Ok(())
}

fn collect_pstore(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let pstore_root = Path::new("/sys/fs/pstore");

    if !pstore_root.exists() {
        let entry = ManifestEntry::new(
            "sys.pstore.status",
            SourceSlug::Sys,
            "pstore",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("/sys/fs/pstore not present on this host")
        .with_limits(ObjectLimits::new(0, 50, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let limits = BoundedTraversalLimits::new(128, 2, 500);
    let read_limits = BoundedReadLimits::new(1_048_576, 200);

    match list_dir_entries(pstore_root, limits) {
        Ok(listing) => {
            let mut total_captured: u64 = 0;

            for entry in &listing.entries {
                if entry.is_dir {
                    continue;
                }

                let source_path = pstore_root.join(&entry.name);
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

                        let entry = ManifestEntry::new(
                            format!("sys.pstore.{}", entry.name),
                            SourceSlug::Sys,
                            "pstore",
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
                        total_captured += 1;
                    }
                    Err(error) => {
                        let status = fs_error_to_manifest_status(&error);
                        let entry = ManifestEntry::new(
                            format!("sys.pstore.{}", entry.name),
                            SourceSlug::Sys,
                            "pstore",
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

                        store.record_object(entry)?;
                    }
                }
            }

            let dir_entry = ManifestEntry::new(
                "sys.pstore.listing",
                SourceSlug::Sys,
                "pstore",
                "listing",
                ObjectKind::DirListing,
                ManifestStatus::Captured,
            )
            .with_bytes(total_captured)
            .with_limits(ObjectLimits::new(
                0,
                limits.timeout_ms,
                listing.entries.len() as u64,
                limits.max_depth,
            ));

            store.record_object(dir_entry)?;
        }
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                "sys.pstore.status",
                SourceSlug::Sys,
                "pstore",
                "status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, 50, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn collect_power(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let power_dir = Path::new("/sys/power");
    if !power_dir.exists() {
        return Ok(());
    }

    let limits = BoundedReadLimits::new(4_096, 100);
    let power_files = ["state", "disk", "pm_test", "wakeup_count"];

    for name in &power_files {
        let source_path = power_dir.join(*name);
        match read_file_bounded(&source_path, limits) {
            Ok(result) => {
                let out_path = output_path(&source_path);
                let bytes = result.content.len() as u64;
                store.write_raw_file(&out_path, &result.content)?;

                let status = if result.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };

                let entry = ManifestEntry::new(
                    format!("sys.power.{name}"),
                    SourceSlug::Sys,
                    "power",
                    *name,
                    ObjectKind::File,
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
                    format!("sys.power.{name}"),
                    SourceSlug::Sys,
                    "power",
                    *name,
                    ObjectKind::File,
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

fn collect_class_net(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let net_dir = Path::new("/sys/class/net");
    if !net_dir.exists() {
        return Ok(());
    }

    collect_bounded_device_tree(
        store,
        net_dir,
        "net",
        &[
            "mtu",
            "flags",
            "address",
            "addr_len",
            "type",
            "operstate",
            "speed",
            "carrier",
            "duplex",
            "dev_port",
        ],
        1_048_576,
        16,
        2,
    )
}

fn collect_block(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let block_dir = Path::new("/sys/block");
    if !block_dir.exists() {
        return Ok(());
    }

    collect_bounded_device_tree(
        store,
        block_dir,
        "block",
        &[
            "size",
            "removable",
            "ro",
            "capability",
            "ext_range",
            "stat",
            "inflight",
        ],
        1_048_576,
        16,
        2,
    )
}

fn collect_bounded_device_tree(
    store: &mut SnapshotStore,
    root: &Path,
    domain: &str,
    attr_files: &[&str],
    max_bytes: u64,
    max_devices: u64,
    max_depth: u32,
) -> Result<(), StoreError> {
    let limits = BoundedTraversalLimits::new(max_devices, max_depth, 500);
    let read_limits = BoundedReadLimits::new(max_bytes, 200);

    let listing = match list_dir_entries(root, limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("sys.{domain}.status"),
                SourceSlug::Sys,
                domain,
                "status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, 50, 1, 0));

            store.record_object(entry)?;
            return Ok(());
        }
    };

    for entry in &listing.entries {
        if !entry.is_dir {
            continue;
        }

        let device_dir = root.join(&entry.name);

        for attr in attr_files {
            let source_path = device_dir.join(attr);
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

                    let entry = ManifestEntry::new(
                        format!("sys.{domain}.{}.{attr}", entry.name),
                        SourceSlug::Sys,
                        domain,
                        format!("{}/{}", entry.name, attr),
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
                }
                Err(error) => {
                    let status = fs_error_to_manifest_status(&error);
                    let entry = ManifestEntry::new(
                        format!("sys.{domain}.{}.{attr}", entry.name),
                        SourceSlug::Sys,
                        domain,
                        format!("{}/{}", entry.name, attr),
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

                    store.record_object(entry)?;
                }
            }
        }

        let symlink_files = ["device", "subsystem"];
        for symlink in &symlink_files {
            let source_path = device_dir.join(symlink);
            match read_link_bounded(&source_path, read_limits) {
                Ok(target) => {
                    let out_path = output_path(&source_path);
                    let content = target.to_string_lossy().into_owned();
                    store.write_raw_file(&out_path, content.as_bytes())?;

                    let entry = ManifestEntry::new(
                        format!("sys.{domain}.{}.{symlink}", entry.name),
                        SourceSlug::Sys,
                        domain,
                        format!("{}/{}", entry.name, symlink),
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

                    store.record_object(entry)?;
                }
                Err(_error) => {}
            }
        }

        let uevent_path = device_dir.join("uevent");
        match read_file_bounded(&uevent_path, BoundedReadLimits::new(4_096, 50)) {
            Ok(result) => {
                let out_path = output_path(&uevent_path);
                let bytes = result.content.len() as u64;
                store.write_raw_file(&out_path, &result.content)?;

                let entry = ManifestEntry::new(
                    format!("sys.{domain}.{}.uevent", entry.name),
                    SourceSlug::Sys,
                    domain,
                    format!("{}/uevent", entry.name),
                    ObjectKind::File,
                    ManifestStatus::Captured,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(4_096, 50, 1, 0));

                store.record_object(entry)?;
            }
            Err(_error) => {}
        }
    }

    Ok(())
}
