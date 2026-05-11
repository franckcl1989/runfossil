#![doc = "Container namespace evidence via /proc/<pid>/ns symlinks."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, read_link_bounded};
use runfossil_store::{ErrorLogEntry, ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

const MAX_NS_BYTES: u64 = 4_096;
const TIMEOUT_MS: u64 = 3_000;
const MAX_CONTAINER_PIDS: u64 = 512;

const NAMESPACES: &[(&str, &str)] = &[
    ("cgroup", "cgroup"),
    ("ipc", "ipc"),
    ("mnt", "mnt"),
    ("net", "net"),
    ("pid", "pid"),
    ("pid_for_children", "pid_for_children"),
    ("time", "time"),
    ("time_for_children", "time_for_children"),
    ("user", "user"),
    ("uts", "uts"),
];

pub(crate) fn collect_namespace_evidence(
    store: &mut SnapshotStore,
    container_pids: &[u32],
) -> Result<(), StoreError> {
    if container_pids.is_empty() {
        return Ok(());
    }

    let limits = BoundedReadLimits::new(MAX_NS_BYTES, TIMEOUT_MS);
    for (captured, &pid) in container_pids.iter().enumerate() {
        if captured as u64 >= MAX_CONTAINER_PIDS {
            let entry = ManifestEntry::new(
                "container.namespace.truncation",
                SourceSlug::Container,
                "namespace",
                "truncation",
                ObjectKind::Metadata,
                ManifestStatus::Truncated,
            )
            .with_reason(format!(
                "namespace capture truncated at {MAX_CONTAINER_PIDS} PIDs"
            ));
            store.record_object(entry)?;
            break;
        }

        capture_pid_namespaces(store, pid, limits)?;
    }

    Ok(())
}

fn capture_pid_namespaces(
    store: &mut SnapshotStore,
    pid: u32,
    limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    let ns_dir = Path::new("/proc").join(pid.to_string()).join("ns");
    if !ns_dir.is_dir() {
        return Ok(());
    }

    for &(ns_name, _) in NAMESPACES {
        let ns_path = ns_dir.join(ns_name);
        match read_link_bounded(&ns_path, limits) {
            Ok(target) => {
                let value = target.to_string_lossy().into_owned();
                let value_bytes = value.as_bytes();
                let out_path = format!("raw/container/ns/{pid}/{ns_name}");

                store.write_raw_file(Path::new(&out_path), value_bytes)?;

                let entry = ManifestEntry::new(
                    format!("container.ns.{pid}.{ns_name}"),
                    SourceSlug::Container,
                    "namespace",
                    ns_name,
                    ObjectKind::Symlink,
                    ManifestStatus::Captured,
                )
                .with_path(&out_path)
                .with_bytes(value_bytes.len() as u64)
                .with_limits(ObjectLimits::new(
                    limits.max_bytes,
                    limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(entry)?;
            }
            Err(error) => {
                let error_log = ErrorLogEntry::new(
                    "0",
                    None::<String>,
                    ManifestStatus::IoError,
                    format!("cannot read namespace {ns_path:?}: {error}"),
                );
                store.log_error(&error_log)?;
            }
        }
    }

    Ok(())
}

pub(crate) fn discover_container_pids() -> Vec<u32> {
    let mut pids = Vec::new();
    discover_pids_from_cgroup(
        Path::new("/sys/fs/cgroup/system.slice"),
        "docker-",
        &mut pids,
    );
    discover_pids_from_cgroup(
        Path::new("/sys/fs/cgroup/system.slice"),
        "containerd-",
        &mut pids,
    );
    discover_pids_from_cgroup(
        Path::new("/sys/fs/cgroup/system.slice"),
        "libpod-",
        &mut pids,
    );
    discover_pids_from_cgroup(
        Path::new("/sys/fs/cgroup/machine.slice"),
        "libpod-",
        &mut pids,
    );
    pids.sort();
    pids.dedup();
    pids
}

fn discover_pids_from_cgroup(base: &Path, marker: &str, pids: &mut Vec<u32>) {
    let entries = match std::fs::read_dir(base) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(marker) {
            continue;
        }
        let procs_file = entry.path().join("cgroup.procs");
        if let Ok(content) = std::fs::read_to_string(&procs_file) {
            for line in content.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    pids.push(pid);
                    if pids.len() as u64 >= MAX_CONTAINER_PIDS {
                        return;
                    }
                }
            }
        }
    }
}
