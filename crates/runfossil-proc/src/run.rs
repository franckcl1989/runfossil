#![forbid(unsafe_code)]
#![doc = "/run runtime state collectors. Linux host state, uses auto-discovery with bounded traversal."]

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{AutoDiscoverConfig, BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

const CONTAINER_RUNTIME_METADATA_PATHS: &[(&str, &str)] = &[
    ("docker.sock", "/run/docker.sock"),
    ("containerd.dir", "/run/containerd"),
    ("containerd.sock", "/run/containerd/containerd.sock"),
    ("crio.dir", "/run/crio"),
    ("crio.sock", "/run/crio/crio.sock"),
    ("runc.dir", "/run/runc"),
    ("podman.dir", "/run/podman"),
    ("podman.sock", "/run/podman/podman.sock"),
    ("kata-containers.dir", "/run/kata-containers"),
    ("gvisor.dir", "/run/gvisor"),
];

pub(crate) fn collect_run(store: &SnapshotStore) -> Result<(), StoreError> {
    let config = AutoDiscoverConfig::run_dir();
    let root = Path::new("/run");

    if !root.exists() {
        return Ok(());
    }

    collect_container_runtime_metadata(store)?;

    let limits = BoundedTraversalLimits::new(config.max_files_per_level, 0, config.timeout_ms);
    let read_limits = BoundedReadLimits::new(config.max_bytes_per_file, config.timeout_ms);

    let listing = match list_dir_entries(root, limits) {
        Ok(l) => l,
        Err(_) => return Ok(()),
    };

    for entry in &listing.entries {
        if config.is_blacklisted(&entry.name) {
            continue;
        }

        let source_path = root.join(&entry.name);

        if entry.is_dir {
            collect_run_tree(store, &source_path, &entry.name, &config, read_limits, 1)?;
        } else {
            collect_run_file(store, &source_path, "top", &entry.name, read_limits)?;
        }
    }

    Ok(())
}

fn collect_container_runtime_metadata(store: &SnapshotStore) -> Result<(), StoreError> {
    for &(id, path) in CONTAINER_RUNTIME_METADATA_PATHS {
        let source_path = Path::new(path);
        let meta = match fs::symlink_metadata(source_path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                let status = match error.kind() {
                    std::io::ErrorKind::PermissionDenied => ManifestStatus::PermissionDenied,
                    _ => ManifestStatus::IoError,
                };
                store.record_object(
                    ManifestEntry::new(
                        format!("run.container_runtime.{id}"),
                        SourceSlug::Run,
                        "container_runtime",
                        path,
                        ObjectKind::MetadataOnly,
                        status,
                    )
                    .with_reason(error.to_string())
                    .with_limits(ObjectLimits::new(0, 100, 1, 0)),
                )?;
                continue;
            }
        };

        let out_path = Path::new("raw/run/container_runtime").join(format!("{id}.metadata"));
        let content = metadata_line(source_path, &meta);
        store.write_raw_file(&out_path, content.as_bytes())?;

        store.record_object(
            ManifestEntry::new(
                format!("run.container_runtime.{id}"),
                SourceSlug::Run,
                "container_runtime",
                path,
                ObjectKind::MetadataOnly,
                ManifestStatus::Captured,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(content.len() as u64)
            .with_limits(ObjectLimits::new(512, 100, 1, 0)),
        )?;
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
    if file_type.is_socket() {
        "socket"
    } else if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_file() {
        "regular"
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

fn collect_run_tree(
    store: &SnapshotStore,
    dir: &Path,
    domain: &str,
    config: &AutoDiscoverConfig,
    read_limits: BoundedReadLimits,
    depth: u32,
) -> Result<(), StoreError> {
    if depth >= config.max_depth {
        return Ok(());
    }

    let limits = BoundedTraversalLimits::new(config.max_files_per_level, 0, config.timeout_ms);

    let listing = match list_dir_entries(dir, limits) {
        Ok(l) => l,
        Err(_) => return Ok(()),
    };

    for entry in &listing.entries {
        if config.is_blacklisted(&entry.name) {
            continue;
        }

        let source_path = dir.join(&entry.name);

        if entry.is_dir && depth + 1 < config.max_depth {
            let sub_domain = format!("{}/{}", domain, entry.name);
            collect_run_tree(
                store,
                &source_path,
                &sub_domain,
                config,
                read_limits,
                depth + 1,
            )?;
        } else if !entry.is_dir {
            collect_run_file(store, &source_path, domain, &entry.name, read_limits)?;
        }
    }

    Ok(())
}

fn collect_run_file(
    store: &SnapshotStore,
    source_path: &Path,
    domain: &str,
    name: &str,
    limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    match read_file_bounded(source_path, limits) {
        Ok(result) => {
            let out_path = output_path(source_path);
            let bytes = result.content.len() as u64;
            store.write_raw_file(&out_path, &result.content)?;

            let status = if result.was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let entry = ManifestEntry::new(
                format!("run.{}.{}", domain.replace('/', "."), name),
                SourceSlug::Run,
                domain,
                name,
                ObjectKind::File,
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
                format!("run.{}.{}", domain.replace('/', "."), name),
                SourceSlug::Run,
                domain,
                name,
                ObjectKind::File,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}
