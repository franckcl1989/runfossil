#![forbid(unsafe_code)]
#![doc = "Sysfs collector: /sys/fs/cgroup detection, pstore capture, class device capture."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{AutoDiscoverConfig, BoundedReadLimits, BoundedTraversalLimits};
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
pub(crate) fn collect_sys(store: &SnapshotStore) -> Result<(), StoreError> {
    record_cgroup_version(store)?;
    collect_cgroup_tree(store)?;
    collect_pstore(store)?;
    collect_power(store)?;
    collect_class_net(store)?;
    collect_block(store)?;
    collect_devices_cpu(store)?;
    collect_devices_node(store)?;
    collect_hwmon(store)?;
    collect_thermal(store)?;
    collect_module_sys(store)?;
    collect_kernel_sys(store)?;
    collect_firmware_sys(store)?;
    collect_power_supply(store)?;
    Ok(())
}

fn record_cgroup_version(store: &SnapshotStore) -> Result<(), StoreError> {
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

fn collect_cgroup_tree(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/fs/cgroup");

    if !root.exists() {
        let entry = ManifestEntry::new(
            "sys.cgroup.status",
            SourceSlug::Sys,
            "cgroup",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("/sys/fs/cgroup not present on this host")
        .with_limits(ObjectLimits::new(0, 50, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let traversal_limits = BoundedTraversalLimits::new(128, 4, 1000);
    let read_limits = BoundedReadLimits::new(262_144, 200);
    walk_cgroup_tree(store, root, root, traversal_limits, read_limits, 0)
}

fn walk_cgroup_tree(
    store: &SnapshotStore,
    base: &Path,
    current: &Path,
    traversal_limits: BoundedTraversalLimits,
    read_limits: BoundedReadLimits,
    depth: u32,
) -> Result<(), StoreError> {
    if depth >= traversal_limits.max_depth {
        return Ok(());
    }

    let listing = match list_dir_entries(current, traversal_limits) {
        Ok(listing) => listing,
        Err(error) => {
            let rel = current.strip_prefix(base).unwrap_or(current);
            let object = if rel.as_os_str().is_empty() {
                "root".to_string()
            } else {
                rel.to_string_lossy().into_owned()
            };
            let id = id_component(&object);
            let status = fs_error_to_manifest_status(&error);
            store.record_object(
                ManifestEntry::new(
                    format!("sys.cgroup.{id}.listing"),
                    SourceSlug::Sys,
                    "cgroup",
                    object,
                    ObjectKind::DirListing,
                    status,
                )
                .with_reason(error.to_string())
                .with_limits(ObjectLimits::new(
                    0,
                    traversal_limits.timeout_ms,
                    traversal_limits.max_files,
                    traversal_limits.max_depth,
                )),
            )?;
            return Ok(());
        }
    };

    for entry in &listing.entries {
        let source_path = current.join(&entry.name);
        if entry.is_dir {
            walk_cgroup_tree(
                store,
                base,
                &source_path,
                traversal_limits,
                read_limits,
                depth + 1,
            )?;
            continue;
        }

        let rel = source_path.strip_prefix(base).unwrap_or(&source_path);
        let object = rel.to_string_lossy().into_owned();
        let id = id_component(&object);

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
                        format!("sys.cgroup.{id}"),
                        SourceSlug::Sys,
                        "cgroup",
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
                    )),
                )?;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                store.record_object(
                    ManifestEntry::new(
                        format!("sys.cgroup.{id}"),
                        SourceSlug::Sys,
                        "cgroup",
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
                    )),
                )?;
            }
        }
    }

    if listing.was_truncated {
        let rel = current.strip_prefix(base).unwrap_or(current);
        let object = if rel.as_os_str().is_empty() {
            "root".to_string()
        } else {
            rel.to_string_lossy().into_owned()
        };
        let id = id_component(&object);
        store.record_object(
            ManifestEntry::new(
                format!("sys.cgroup.{id}.listing"),
                SourceSlug::Sys,
                "cgroup",
                object,
                ObjectKind::DirListing,
                ManifestStatus::Truncated,
            )
            .with_reason("cgroup directory listing reached file-count limit")
            .with_limits(ObjectLimits::new(
                0,
                traversal_limits.timeout_ms,
                traversal_limits.max_files,
                traversal_limits.max_depth,
            )),
        )?;
    }

    Ok(())
}

fn collect_pstore(store: &SnapshotStore) -> Result<(), StoreError> {
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

fn collect_power(store: &SnapshotStore) -> Result<(), StoreError> {
    let power_dir = Path::new("/sys/power");
    if !power_dir.exists() {
        return Ok(());
    }

    let limits = BoundedReadLimits::new(4_096, 100);
    let power_files = ["pm_test", "wakeup_count"];

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

fn collect_class_net(store: &SnapshotStore) -> Result<(), StoreError> {
    let net_dir = Path::new("/sys/class/net");
    if !net_dir.exists() {
        return Ok(());
    }

    collect_bounded_device_tree(store, net_dir, "net", &[], 1_048_576, 16, 2)
}

fn collect_block(store: &SnapshotStore) -> Result<(), StoreError> {
    let block_dir = Path::new("/sys/block");
    if !block_dir.exists() {
        return Ok(());
    }

    collect_bounded_device_tree(store, block_dir, "block", &[], 1_048_576, 16, 2)
}

fn collect_bounded_device_tree(
    store: &SnapshotStore,
    root: &Path,
    domain: &str,
    attr_files: &[&str],
    max_bytes: u64,
    max_devices: u64,
    max_depth: u32,
) -> Result<(), StoreError> {
    let limits = BoundedTraversalLimits::new(max_devices, max_depth, 500);
    let read_limits = BoundedReadLimits::new(max_bytes, 200);
    let config = AutoDiscoverConfig::sys_device_class();
    // Use auto-discovery when attr_files is empty
    let auto_discover = attr_files.is_empty();

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

        if auto_discover {
            // Auto-discover: list device directory, read all non-blacklisted files
            let dev_limit = BoundedTraversalLimits::new(128, config.max_depth, config.timeout_ms);
            if let Ok(dev_listing) = list_dir_entries(&device_dir, dev_limit) {
                for dev_entry in &dev_listing.entries {
                    if dev_entry.is_dir {
                        continue;
                    }
                    if config.is_blacklisted(&dev_entry.name) {
                        continue;
                    }
                    let source_path = device_dir.join(&dev_entry.name);
                    sys_read_and_record(
                        store,
                        &source_path,
                        domain,
                        &entry.name,
                        &dev_entry.name,
                        read_limits,
                        ObjectKind::File,
                    )?;
                }
            }
        } else {
            // Hardcoded attribute list (legacy mode, kept for specific cases)
            for attr in attr_files {
                let source_path = device_dir.join(attr);
                sys_read_and_record(
                    store,
                    &source_path,
                    domain,
                    &entry.name,
                    attr,
                    read_limits,
                    ObjectKind::File,
                )?;
            }

            // Always try symlink resolution for device/subsystem
            for symlink in &["device", "subsystem"] {
                let source_path = device_dir.join(symlink);
                if let Ok(target) = read_link_bounded(&source_path, read_limits) {
                    let out_path = output_path(&source_path);
                    let content = target.to_string_lossy().into_owned();
                    if store.write_raw_file(&out_path, content.as_bytes()).is_ok() {
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
                }
            }
        }
    }

    Ok(())
}

fn sys_read_and_record(
    store: &SnapshotStore,
    source_path: &Path,
    domain: &str,
    device_name: &str,
    attr_name: &str,
    limits: BoundedReadLimits,
    kind: ObjectKind,
) -> Result<(), StoreError> {
    match kind {
        ObjectKind::Symlink => match read_link_bounded(source_path, limits) {
            Ok(target) => {
                let out_path = output_path(source_path);
                let content = target.to_string_lossy().into_owned();
                let bytes = content.len() as u64;
                store.write_raw_file(&out_path, content.as_bytes())?;
                let entry = ManifestEntry::new(
                    format!("sys.{domain}.{device_name}.{attr_name}"),
                    SourceSlug::Sys,
                    domain,
                    format!("{device_name}/{attr_name}"),
                    kind,
                    ManifestStatus::Captured,
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
                    format!("sys.{domain}.{device_name}.{attr_name}"),
                    SourceSlug::Sys,
                    domain,
                    format!("{device_name}/{attr_name}"),
                    kind,
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
        },
        _ => match read_file_bounded(source_path, limits) {
            Ok(r) => {
                let out_path = output_path(source_path);
                let bytes = r.content.len() as u64;
                store.write_raw_file(&out_path, &r.content)?;

                let status = if r.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };
                let entry = ManifestEntry::new(
                    format!("sys.{domain}.{device_name}.{attr_name}"),
                    SourceSlug::Sys,
                    domain,
                    format!("{device_name}/{attr_name}"),
                    kind,
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
                    format!("sys.{domain}.{device_name}.{attr_name}"),
                    SourceSlug::Sys,
                    domain,
                    format!("{device_name}/{attr_name}"),
                    kind,
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
        },
    }
    Ok(())
}

fn collect_devices_cpu(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/devices/system/cpu");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "devices_cpu", &[], 1_048_576, 64, 3)
}

fn collect_devices_node(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/devices/system/node");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "devices_node", &[], 1_048_576, 32, 3)
}

fn collect_hwmon(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/class/hwmon");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "hwmon", &[], 1_048_576, 64, 2)
}

fn collect_thermal(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/class/thermal");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "thermal", &[], 1_048_576, 32, 2)
}

fn collect_module_sys(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/module");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "module", &[], 1_048_576, 256, 3)
}

fn collect_kernel_sys(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/kernel");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "kernel", &[], 1_048_576, 64, 3)
}

fn collect_firmware_sys(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/firmware");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "firmware", &[], 1_048_576, 128, 3)
}

fn collect_power_supply(store: &SnapshotStore) -> Result<(), StoreError> {
    let root = Path::new("/sys/class/power_supply");
    if !root.exists() {
        return Ok(());
    }
    collect_bounded_device_tree(store, root, "power_supply", &[], 1_048_576, 32, 2)
}

fn id_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' => output.push(byte as char),
            b'A'..=b'Z' => output.push((byte + 32) as char),
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
        "root".to_string()
    } else {
        output
    }
}
