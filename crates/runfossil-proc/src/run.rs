#![forbid(unsafe_code)]
#![doc = "/run runtime state collectors. Linux host state, uses auto-discovery with bounded traversal."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{AutoDiscoverConfig, BoundedReadLimits, BoundedTraversalLimits};
use runfossil_fs::{list_dir_entries, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::{fs_error_to_manifest_status, output_path};

pub(crate) fn collect_run(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let config = AutoDiscoverConfig::run_dir();
    let root = Path::new("/run");

    if !root.exists() {
        return Ok(());
    }

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

fn collect_run_tree(
    store: &mut SnapshotStore,
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
    store: &mut SnapshotStore,
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
