#![doc = "Container cgroup evidence collection."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{
    BoundedReadLimits, BoundedTraversalLimits, list_dir_entries, read_file_bounded,
};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

const CGROUP_ROOT: &str = "/sys/fs/cgroup";
const MAX_CGROUP_FILES: u64 = 128;
const MAX_CGROUP_DEPTH: u32 = 3;
const MAX_BYTES_PER_FILE: u64 = 65_536;
const TIMEOUT_MS: u64 = 5_000;

const CONTAINER_MARKERS: &[&str] = &[
    "docker-",
    "docker/",
    "containerd-",
    "containerd/",
    "crio-",
    "crio/",
    "kubepods-",
    "kubepods/",
    "libpod-",
];

pub(crate) fn collect_cgroup_evidence(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let cgroup_root = Path::new(CGROUP_ROOT);
    if !cgroup_root.is_dir() {
        return Ok(());
    }

    let limits = BoundedTraversalLimits::new(MAX_CGROUP_FILES, MAX_CGROUP_DEPTH, TIMEOUT_MS);
    let file_limits = BoundedReadLimits::new(MAX_BYTES_PER_FILE, TIMEOUT_MS);

    let listing =
        match list_dir_entries(cgroup_root, BoundedTraversalLimits::new(32, 1, TIMEOUT_MS)) {
            Ok(listing) => listing,
            Err(_) => return Ok(()),
        };

    for entry in &listing.entries {
        let controller_path = cgroup_root.join(&entry.name);
        if !controller_path.is_dir() {
            continue;
        }
        scan_cgroup_controller(store, &controller_path, &entry.name, limits, file_limits)?;
    }

    Ok(())
}

fn scan_cgroup_controller(
    store: &mut SnapshotStore,
    path: &Path,
    controller_name: &str,
    limits: BoundedTraversalLimits,
    file_limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    let listing = match list_dir_entries(path, limits) {
        Ok(listing) => listing,
        Err(_) => return Ok(()),
    };

    for entry in &listing.entries {
        if is_container_related(&entry.name) {
            let cgroup_path = path.join(&entry.name);
            capture_cgroup_tree(
                store,
                &cgroup_path,
                controller_name,
                &entry.name,
                file_limits,
            )?;
        }
    }

    Ok(())
}

fn is_container_related(name: &str) -> bool {
    CONTAINER_MARKERS
        .iter()
        .any(|m| name.starts_with(m) || name.contains(m))
}

fn capture_cgroup_tree(
    store: &mut SnapshotStore,
    path: &Path,
    controller: &str,
    name: &str,
    file_limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    let sub_limits = BoundedTraversalLimits::new(MAX_CGROUP_FILES, MAX_CGROUP_DEPTH, TIMEOUT_MS);

    let listing = match list_dir_entries(path, sub_limits) {
        Ok(listing) => listing,
        Err(_) => {
            record_cgroup_metadata(store, path, controller, name, file_limits)?;
            return Ok(());
        }
    };

    let key_stats = &[
        "memory.current",
        "memory.max",
        "memory.stat",
        "cpu.stat",
        "cpu.pressure",
        "pids.current",
        "pids.max",
        "io.stat",
        "cgroup.procs",
        "cgroup.controllers",
        "cgroup.events",
    ];

    for stat in key_stats {
        let stat_path = path.join(stat);
        read_and_store_cgroup_file(store, &stat_path, controller, name, stat, file_limits)?;
    }

    let entry = ManifestEntry::new(
        format!("container.cgroup.{controller}.{name}.listing"),
        SourceSlug::Container,
        "cgroup",
        format!("{controller}/{name}"),
        ObjectKind::DirListing,
        ManifestStatus::Captured,
    )
    .with_path(format!("raw/container/cgroup/{controller}/{name}"))
    .with_bytes(listing.entries.len() as u64)
    .with_limits(ObjectLimits::new(
        0,
        TIMEOUT_MS,
        MAX_CGROUP_FILES,
        MAX_CGROUP_DEPTH,
    ));

    store.record_object(entry)?;

    for sub in &listing.entries {
        if !sub.is_dir {
            let sub_path = path.join(&sub.name);
            read_and_store_cgroup_file(
                store,
                &sub_path,
                controller,
                &format!("{name}/{}", sub.name),
                &sub.name,
                file_limits,
            )?;
        }
    }

    Ok(())
}

fn record_cgroup_metadata(
    store: &mut SnapshotStore,
    path: &Path,
    controller: &str,
    name: &str,
    file_limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    for stat in &[
        "memory.current",
        "memory.max",
        "cpu.stat",
        "pids.current",
        "cgroup.procs",
    ] {
        let stat_path = path.join(stat);
        read_and_store_cgroup_file(store, &stat_path, controller, name, stat, file_limits)?;
    }

    let entry = ManifestEntry::new(
        format!("container.cgroup.{controller}.{name}"),
        SourceSlug::Container,
        "cgroup",
        format!("{controller}/{name}"),
        ObjectKind::FileSet,
        ManifestStatus::Captured,
    )
    .with_path(format!("raw/container/cgroup/{controller}/{name}"))
    .with_limits(ObjectLimits::new(
        MAX_BYTES_PER_FILE,
        TIMEOUT_MS,
        MAX_CGROUP_FILES,
        0,
    ));

    store.record_object(entry)?;
    Ok(())
}

fn read_and_store_cgroup_file(
    store: &mut SnapshotStore,
    path: &Path,
    controller: &str,
    name: &str,
    filename: &str,
    limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    let out_path = format!("raw/container/cgroup/{controller}/{name}/{filename}");

    if let Ok(result) = read_file_bounded(path, limits) {
        let bytes = result.content.len() as u64;
        store.write_raw_file(Path::new(&out_path), &result.content)?;

        let status = if result.was_truncated {
            ManifestStatus::Truncated
        } else {
            ManifestStatus::Captured
        };

        let entry = ManifestEntry::new(
            format!("container.cgroup.{controller}.{name}.{filename}"),
            SourceSlug::Container,
            "cgroup",
            filename,
            ObjectKind::File,
            status,
        )
        .with_path(&out_path)
        .with_bytes(bytes)
        .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

        store.record_object(entry)?;
    }

    Ok(())
}
