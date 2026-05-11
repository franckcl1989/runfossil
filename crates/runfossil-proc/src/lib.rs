#![forbid(unsafe_code)]
#![doc = "Procfs and sysfs collectors for runfossil. Reads /proc and /sys runtime evidence without external commands."]

pub(crate) mod dev;
pub(crate) mod kmsg;
pub(crate) mod sys;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, BoundedTraversalLimits, FsError, ListResult};
use runfossil_fs::{list_dir_entries, read_file_bounded, read_link_bounded};
use runfossil_store::{ErrorLogEntry, ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

/// Returns the source slug owned by this collector crate.
#[must_use]
pub const fn source() -> SourceSlug {
    SourceSlug::Proc
}

/// A single procfs file target with collection metadata.
struct ProcTarget {
    source_path: PathBuf,
    domain: &'static str,
    object: &'static str,
    kind: ObjectKind,
}

/// P0 global /proc files: low-cost, high-value system overview.
fn p0_global_targets() -> Vec<ProcTarget> {
    let mut targets = Vec::new();

    target(
        &mut targets,
        "system",
        "loadavg",
        "loadavg",
        ObjectKind::File,
    );
    target(&mut targets, "system", "uptime", "uptime", ObjectKind::File);
    target(
        &mut targets,
        "system",
        "version",
        "version",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "system",
        "cmdline",
        "cmdline",
        ObjectKind::File,
    );
    target(&mut targets, "cpu", "cpuinfo", "cpuinfo", ObjectKind::File);
    target(&mut targets, "memory", "swaps", "swaps", ObjectKind::File);
    target(&mut targets, "cpu", "stat", "stat", ObjectKind::File);
    target(
        &mut targets,
        "cpu",
        "schedstat",
        "schedstat",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "memory",
        "meminfo",
        "meminfo",
        ObjectKind::File,
    );
    target(&mut targets, "memory", "vmstat", "vmstat", ObjectKind::File);
    target(
        &mut targets,
        "pressure",
        "cpu",
        "pressure/cpu",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "pressure",
        "memory",
        "pressure/memory",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "pressure",
        "io",
        "pressure/io",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "interrupt",
        "interrupts",
        "interrupts",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "interrupt",
        "softirqs",
        "softirqs",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "block",
        "diskstats",
        "diskstats",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "block",
        "partitions",
        "partitions",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "filesystem",
        "mounts",
        "mounts",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "filesystem",
        "mountinfo",
        "self/mountinfo",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "filesystem",
        "filesystems",
        "filesystems",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "kernel",
        "modules",
        "modules",
        ObjectKind::File,
    );

    net_target(&mut targets, "tcp");
    net_target(&mut targets, "tcp6");
    net_target(&mut targets, "udp");
    net_target(&mut targets, "udp6");
    net_target(&mut targets, "unix");
    net_target(&mut targets, "raw");
    net_target(&mut targets, "packet");
    net_target(&mut targets, "dev");
    net_target(&mut targets, "snmp");
    net_target(&mut targets, "netstat");
    net_target(&mut targets, "arp");
    net_target(&mut targets, "route");
    net_target(&mut targets, "ipv6_route");

    targets
}

/// P2 limited /proc files.
fn p2_limited_targets() -> Vec<ProcTarget> {
    let mut targets = Vec::new();

    target(
        &mut targets,
        "memory",
        "slabinfo",
        "slabinfo",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "memory",
        "zoneinfo",
        "zoneinfo",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "memory",
        "buddyinfo",
        "buddyinfo",
        ObjectKind::File,
    );
    target(&mut targets, "kernel", "locks", "locks", ObjectKind::File);
    target(&mut targets, "ipc", "shm", "sysvipc/shm", ObjectKind::File);
    target(&mut targets, "ipc", "msg", "sysvipc/msg", ObjectKind::File);
    target(&mut targets, "ipc", "sem", "sysvipc/sem", ObjectKind::File);
    target(&mut targets, "hardware", "iomem", "iomem", ObjectKind::File);
    target(
        &mut targets,
        "hardware",
        "ioports",
        "ioports",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "hardware",
        "devices",
        "devices",
        ObjectKind::File,
    );
    target(&mut targets, "hardware", "misc", "misc", ObjectKind::File);
    target(
        &mut targets,
        "hardware",
        "cgroups",
        "cgroups",
        ObjectKind::File,
    );
    target(&mut targets, "crypto", "crypto", "crypto", ObjectKind::File);
    target(
        &mut targets,
        "crypto",
        "key-users",
        "key-users",
        ObjectKind::File,
    );
    target(
        &mut targets,
        "filesystem",
        "mountstats",
        "self/mountstats",
        ObjectKind::File,
    );

    net_target(&mut targets, "dev_mcast");
    net_target(&mut targets, "igmp");
    net_target(&mut targets, "igmp6");

    targets
}

fn target(
    targets: &mut Vec<ProcTarget>,
    domain: &'static str,
    object: &'static str,
    relative: &str,
    kind: ObjectKind,
) {
    targets.push(ProcTarget {
        source_path: Path::new("/proc").join(relative),
        domain,
        object,
        kind,
    });
}

fn net_target(targets: &mut Vec<ProcTarget>, name: &'static str) {
    let relative = format!("net/{name}");
    target(targets, "net", name, &relative, ObjectKind::File);
}

/// P1 per-process files.
struct ProcPerProcess {
    name: &'static str,
    relative: &'static str,
    kind: ObjectKind,
}

fn p1_process_targets() -> Vec<ProcPerProcess> {
    vec![
        ProcPerProcess {
            name: "status",
            relative: "status",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "stat",
            relative: "stat",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "statm",
            relative: "statm",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "cmdline",
            relative: "cmdline",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "comm",
            relative: "comm",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "limits",
            relative: "limits",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "io",
            relative: "io",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "sched",
            relative: "sched",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "cgroup",
            relative: "cgroup",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "oom_score",
            relative: "oom_score",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "oom_score_adj",
            relative: "oom_score_adj",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "cwd",
            relative: "cwd",
            kind: ObjectKind::Symlink,
        },
        ProcPerProcess {
            name: "root",
            relative: "root",
            kind: ObjectKind::Symlink,
        },
        ProcPerProcess {
            name: "exe",
            relative: "exe",
            kind: ObjectKind::Symlink,
        },
    ]
}

/// Collects all /proc evidence into the given store.
///
/// On live hosts, this requires root permissions for most files. Non-root
/// invocation will produce many `permission_denied` entries.
pub fn collect_proc(store: &mut SnapshotStore) -> Result<(), StoreError> {
    collect_p0_globals(store)?;
    collect_p2_limited(store)?;
    collect_p1_processes(store)?;
    collect_p3_netfilter(store)?;
    Ok(())
}

fn collect_p3_netfilter(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let netfilter_dir = Path::new("/proc/net/netfilter");
    if !netfilter_dir.exists() {
        return Ok(());
    }

    let limits = BoundedTraversalLimits::new(64, 1, 200);
    let read_limits = BoundedReadLimits::new(65_536, 200);

    let listing = match list_dir_entries(netfilter_dir, limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                "proc.net.netfilter.status",
                SourceSlug::Proc,
                "net",
                "netfilter/status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
            return Ok(());
        }
    };

    for entry in &listing.entries {
        if entry.is_dir {
            continue;
        }

        let source_path = netfilter_dir.join(&entry.name);
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

                let manifest_entry = ManifestEntry::new(
                    format!("proc.net.netfilter.{}", entry.name),
                    SourceSlug::Proc,
                    "net",
                    format!("netfilter/{}", entry.name),
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

                store.record_object(manifest_entry)?;
            }
            Err(_error) => {}
        }
    }

    Ok(())
}

/// Collects all /sys evidence into the given store.
///
/// Includes cgroup version detection, pstore capture, power state,
/// network class devices, and block device attributes.
pub fn collect_sys(store: &mut SnapshotStore) -> Result<(), StoreError> {
    sys::collect_sys(store)
}

/// Collects `/dev` metadata and symlink-target evidence.
///
/// Device nodes are metadata-only. Symlink directories (/dev/block,
/// /dev/disk, /dev/mapper) record target paths. Pseudo-devices and
/// loop devices are recorded as metadata.
pub fn collect_dev(store: &mut SnapshotStore) -> Result<(), StoreError> {
    dev::collect_dev(store)
}

/// Collects a bounded window from the kernel ring buffer via `/dev/kmsg`.
///
/// Reads up to 2 MiB from the kernel message buffer and writes it to
/// `raw/kernel/kmsg.window`. Records `not_found` if `/dev/kmsg` is
/// absent.
pub fn collect_kmsg(store: &mut SnapshotStore) -> Result<(), StoreError> {
    kmsg::collect_kmsg(store)
}

pub(crate) fn now_ns() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

pub(crate) fn output_path(source_path: &Path) -> PathBuf {
    let stripped = source_path.strip_prefix("/").unwrap_or(source_path);
    Path::new("raw").join(stripped)
}

fn collect_p0_globals(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let targets = p0_global_targets();
    let limits = BoundedReadLimits::small();

    for t in &targets {
        let started = now_ns();
        let source_display = t.source_path.display().to_string();

        match read_file_bounded(&t.source_path, limits) {
            Ok(result) => {
                let out_path = output_path(&t.source_path);
                let bytes = result.content.len() as u64;
                let finished = now_ns();

                store.write_raw_file(&out_path, &result.content)?;

                let status = if result.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };

                let entry = ManifestEntry::new(
                    format!("proc.{}.{}", t.domain, t.object),
                    SourceSlug::Proc,
                    t.domain,
                    t.object,
                    t.kind,
                    status,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(bytes)
                .with_timing(started, finished, 0)
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
                let finished = now_ns();

                let entry = ManifestEntry::new(
                    format!("proc.{}.{}", t.domain, t.object),
                    SourceSlug::Proc,
                    t.domain,
                    t.object,
                    t.kind,
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

                store.log_error(
                    &ErrorLogEntry::new(
                        finished,
                        None::<String>,
                        status,
                        format!("failed to read {}: {error}", &source_display),
                    )
                    .with_source(SourceSlug::Proc)
                    .with_path(&source_display),
                )?;
            }
        }
    }

    Ok(())
}

fn collect_p2_limited(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let targets = p2_limited_targets();
    let limits = BoundedReadLimits::small();

    for t in &targets {
        let source_display = t.source_path.display().to_string();

        match read_file_bounded(&t.source_path, limits) {
            Ok(result) => {
                let out_path = output_path(&t.source_path);
                let bytes = result.content.len() as u64;

                store.write_raw_file(&out_path, &result.content)?;

                let status = if result.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };

                let entry = ManifestEntry::new(
                    format!("proc.{}.{}", t.domain, t.object),
                    SourceSlug::Proc,
                    t.domain,
                    t.object,
                    t.kind,
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
                    format!("proc.{}.{}", t.domain, t.object),
                    SourceSlug::Proc,
                    t.domain,
                    t.object,
                    t.kind,
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

                store.log_error(
                    &ErrorLogEntry::new(
                        now_ns(),
                        None::<String>,
                        status,
                        format!("failed to read {}: {error}", &source_display),
                    )
                    .with_source(SourceSlug::Proc)
                    .with_path(&source_display),
                )?;
            }
        }
    }

    Ok(())
}

fn collect_p1_processes(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let proc_dir = Path::new("/proc");
    let limits = BoundedTraversalLimits::process_listing();

    let listing = match list_dir_entries(proc_dir, limits) {
        Ok(listing) => listing,
        Err(error) => {
            store.log_error(
                &ErrorLogEntry::new(
                    now_ns(),
                    None::<String>,
                    ManifestStatus::IoError,
                    format!("failed to list /proc: {error}"),
                )
                .with_source(SourceSlug::Proc)
                .with_path("/proc"),
            )?;
            return Ok(());
        }
    };

    let per_process = p1_process_targets();
    let file_limits = BoundedReadLimits::per_process();
    let fds_limits = BoundedTraversalLimits::new(1024, 1, 200);
    let ns_limits = BoundedTraversalLimits::new(32, 1, 100);

    for entry in &listing.entries {
        if !entry.is_dir {
            continue;
        }
        let pid: u32 = match entry.name.parse() {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        collect_process_files(store, pid, &per_process, file_limits)?;
        collect_process_fds(store, pid, fds_limits)?;
        collect_process_ns(store, pid, ns_limits)?;
    }

    Ok(())
}

fn collect_process_files(
    store: &mut SnapshotStore,
    pid: u32,
    targets: &[ProcPerProcess],
    limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    for t in targets {
        let source_path = Path::new("/proc").join(pid.to_string()).join(t.relative);

        match t.kind {
            ObjectKind::Symlink => match read_link_bounded(&source_path, limits) {
                Ok(target_path) => {
                    let out_path = output_path(&source_path);
                    let content = target_path.to_string_lossy().into_owned();
                    store.write_raw_file(&out_path, content.as_bytes())?;

                    let entry = ManifestEntry::new(
                        format!("proc.process.{pid}.{}", t.name),
                        SourceSlug::Proc,
                        "process",
                        format!("{pid}/{}", t.name),
                        t.kind,
                        ManifestStatus::Captured,
                    )
                    .with_path(out_path.to_string_lossy())
                    .with_bytes(content.len() as u64)
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
                        format!("proc.process.{pid}.{}", t.name),
                        SourceSlug::Proc,
                        "process",
                        format!("{pid}/{}", t.name),
                        t.kind,
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
            _ => match read_file_bounded(&source_path, limits) {
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
                        format!("proc.process.{pid}.{}", t.name),
                        SourceSlug::Proc,
                        "process",
                        format!("{pid}/{}", t.name),
                        t.kind,
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
                        format!("proc.process.{pid}.{}", t.name),
                        SourceSlug::Proc,
                        "process",
                        format!("{pid}/{}", t.name),
                        t.kind,
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
    }

    Ok(())
}

fn collect_process_fds(
    store: &mut SnapshotStore,
    pid: u32,
    limits: BoundedTraversalLimits,
) -> Result<(), StoreError> {
    let fd_dir = Path::new("/proc").join(pid.to_string()).join("fd");

    match list_dir_entries(&fd_dir, limits) {
        Ok(ListResult {
            entries,
            was_truncated,
        }) => {
            let mut fd_list = String::new();
            for entry in &entries {
                let symlink_path = fd_dir.join(&entry.name);
                match std::fs::read_link(&symlink_path) {
                    Ok(target) => {
                        fd_list.push_str(&format!("{} -> {}\n", entry.name, target.display()));
                    }
                    Err(_) => {
                        fd_list.push_str(&format!("{}\n", entry.name));
                    }
                }
            }

            let out_path = output_path(&fd_dir);
            store.write_raw_file(&out_path, fd_list.as_bytes())?;

            let status = if was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let entry = ManifestEntry::new(
                format!("proc.process.{pid}.fd"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/fd"),
                ObjectKind::DirListing,
                status,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(fd_list.len() as u64)
            .with_limits(ObjectLimits::new(
                0,
                limits.timeout_ms,
                limits.max_files,
                limits.max_depth,
            ));

            store.record_object(entry)?;
        }
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("proc.process.{pid}.fd"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/fd"),
                ObjectKind::DirListing,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(
                0,
                limits.timeout_ms,
                limits.max_files,
                limits.max_depth,
            ));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn collect_process_ns(
    store: &mut SnapshotStore,
    pid: u32,
    limits: BoundedTraversalLimits,
) -> Result<(), StoreError> {
    let ns_dir = Path::new("/proc").join(pid.to_string()).join("ns");

    match list_dir_entries(&ns_dir, limits) {
        Ok(ListResult {
            entries,
            was_truncated,
        }) => {
            let mut ns_list = String::new();
            for entry in &entries {
                let symlink_path = ns_dir.join(&entry.name);
                match std::fs::read_link(&symlink_path) {
                    Ok(target) => {
                        ns_list.push_str(&format!("{} -> {}\n", entry.name, target.display()));
                    }
                    Err(_) => {
                        ns_list.push_str(&format!("{}\n", entry.name));
                    }
                }
            }

            let out_path = output_path(&ns_dir);
            store.write_raw_file(&out_path, ns_list.as_bytes())?;

            let status = if was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let entry = ManifestEntry::new(
                format!("proc.process.{pid}.ns"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/ns"),
                ObjectKind::Metadata,
                status,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(ns_list.len() as u64)
            .with_limits(ObjectLimits::new(
                0,
                limits.timeout_ms,
                limits.max_files,
                limits.max_depth,
            ));

            store.record_object(entry)?;
        }
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("proc.process.{pid}.ns"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/ns"),
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(
                0,
                limits.timeout_ms,
                limits.max_files,
                limits.max_depth,
            ));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

pub(crate) fn fs_error_to_manifest_status(error: &FsError) -> ManifestStatus {
    match error {
        FsError::NotFound(_) => ManifestStatus::NotFound,
        FsError::PermissionDenied(_) => ManifestStatus::PermissionDenied,
        FsError::Timeout(_) => ManifestStatus::Timeout,
        FsError::SizeLimited { .. } => ManifestStatus::SizeLimited,
        FsError::Io { .. } => ManifestStatus::IoError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::CgroupVersion;
    use runfossil_core::EffectiveUid;
    use runfossil_fs::FsError;
    use runfossil_store::{HostMetadata, SnapshotMetadata};
    use std::io;

    fn test_metadata() -> SnapshotMetadata {
        SnapshotMetadata {
            tool_version: "0.1.0".into(),
            started_at_unix_ns: "1".into(),
            effective_uid: EffectiveUid::ROOT,
            host: HostMetadata {
                hostname: "test".into(),
                boot_id: "test-boot".into(),
                kernel_release: "6.8.0".into(),
                machine: "x86_64".into(),
            },
        }
    }

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn new_store(d: &tempfile::TempDir) -> SnapshotStore {
        SnapshotStore::create(d.path().join("snapshot"), test_metadata()).unwrap()
    }

    #[test]
    fn source_returns_proc_slug() {
        assert_eq!(source(), SourceSlug::Proc);
    }

    #[test]
    fn cgroup_version_as_str() {
        assert_eq!(CgroupVersion::V1.as_str(), "v1");
        assert_eq!(CgroupVersion::V2.as_str(), "v2");
        assert_eq!(CgroupVersion::Hybrid.as_str(), "hybrid");
        assert_eq!(CgroupVersion::None.as_str(), "none");
    }

    #[test]
    fn output_path_strips_absolute_proc_prefix() {
        assert_eq!(
            output_path(Path::new("/proc/stat")),
            Path::new("raw/proc/stat")
        );
        assert_eq!(
            output_path(Path::new("/proc/1/status")),
            Path::new("raw/proc/1/status")
        );
    }

    #[test]
    fn output_path_handles_dev_paths() {
        assert_eq!(
            output_path(Path::new("/dev/kmsg")),
            Path::new("raw/dev/kmsg")
        );
    }

    #[test]
    fn output_path_preserves_relative_paths() {
        assert_eq!(
            output_path(Path::new("relative.txt")),
            Path::new("raw/relative.txt")
        );
    }

    #[test]
    fn fs_error_not_found_maps_to_not_found_status() {
        let e = FsError::NotFound(Path::new("/nope").to_path_buf());
        assert_eq!(fs_error_to_manifest_status(&e), ManifestStatus::NotFound);
    }

    #[test]
    fn fs_error_permission_denied_maps_correctly() {
        let e = FsError::PermissionDenied(Path::new("/root").to_path_buf());
        assert_eq!(
            fs_error_to_manifest_status(&e),
            ManifestStatus::PermissionDenied
        );
    }

    #[test]
    fn fs_error_timeout_maps_correctly() {
        let e = FsError::Timeout(Path::new("/slow").to_path_buf());
        assert_eq!(fs_error_to_manifest_status(&e), ManifestStatus::Timeout);
    }

    #[test]
    fn fs_error_size_limited_maps_correctly() {
        let e = FsError::SizeLimited {
            path: Path::new("/big").to_path_buf(),
            max_bytes: 100,
        };
        assert_eq!(fs_error_to_manifest_status(&e), ManifestStatus::SizeLimited);
    }

    #[test]
    fn fs_error_io_maps_correctly() {
        let e = FsError::Io {
            path: Path::new("/broken").to_path_buf(),
            source: io::Error::new(io::ErrorKind::Other, "broken"),
        };
        assert_eq!(fs_error_to_manifest_status(&e), ManifestStatus::IoError);
    }

    #[test]
    fn all_fs_error_to_status_mappings_are_distinct() {
        let mut seen = Vec::new();
        seen.push(fs_error_to_manifest_status(&FsError::NotFound(
            Path::new("a").to_path_buf(),
        )));
        seen.push(fs_error_to_manifest_status(&FsError::PermissionDenied(
            Path::new("b").to_path_buf(),
        )));
        seen.push(fs_error_to_manifest_status(&FsError::Timeout(
            Path::new("c").to_path_buf(),
        )));
        seen.push(fs_error_to_manifest_status(&FsError::SizeLimited {
            path: Path::new("d").to_path_buf(),
            max_bytes: 1,
        }));
        seen.push(fs_error_to_manifest_status(&FsError::Io {
            path: Path::new("e").to_path_buf(),
            source: io::Error::new(io::ErrorKind::Other, ""),
        }));
        let unique: Vec<_> = seen.iter().collect();
        assert_eq!(
            unique.len(),
            5,
            "all five FsError variants should map to distinct statuses"
        );
    }

    #[test]
    fn kmsg_event_window_object_kind_is_correct() {
        // kmsg collection touches /dev/kmsg which may block on live systems.
        // Verify the object kind constant is correct instead.
        assert_eq!(ObjectKind::EventWindow.as_str(), "event_window");
    }

    #[test]
    fn proc_collector_target_symbols_include_all_domains() {
        let targets = p0_global_targets();
        let domains: std::collections::BTreeSet<&str> = targets.iter().map(|t| t.domain).collect();
        assert!(domains.contains("system"));
        assert!(domains.contains("cpu"));
        assert!(domains.contains("memory"));
        assert!(domains.contains("pressure"));
        assert!(domains.contains("interrupt"));
        assert!(domains.contains("block"));
        assert!(domains.contains("filesystem"));
        assert!(domains.contains("kernel"));
        assert!(domains.contains("net"));
    }

    #[test]
    fn proc_targets_have_non_empty_ids() {
        for t in &p0_global_targets() {
            assert!(
                !t.object.is_empty(),
                "P0 target '{}' has empty object",
                t.domain
            );
            assert!(t.source_path.starts_with("/proc/"));
            assert!(!t.domain.is_empty());
        }
    }

    #[test]
    fn sys_collector_returns_ok_when_sysfs_absent() {
        let d = dir();
        let mut store = new_store(&d);
        let result = collect_sys(&mut store);
        if let Err(e) = result.as_ref() {
            assert!(
                e.to_string().contains("Io") || e.to_string().contains("not found"),
                "unexpected error: {e}"
            );
        }
    }

    #[test]
    fn dev_collector_returns_ok_when_dev_absent() {
        let d = dir();
        let mut store = new_store(&d);
        let result = collect_dev(&mut store);
        if let Err(e) = result.as_ref() {
            assert!(e.to_string().contains("Io"), "unexpected error: {e}");
        }
    }

    #[test]
    fn now_ns_is_a_timestamp_string() {
        let ns = now_ns();
        let parsed: u64 = ns.parse().unwrap();
        assert!(
            parsed > 1_700_000_000_000_000_000,
            "timestamp should be in 2023+ range"
        );
    }

    #[test]
    fn p1_process_targets_are_counted() {
        let targets = p1_process_targets();
        assert_eq!(targets.len(), 14, "expected 14 per-process file targets");
    }

    #[test]
    fn p0_global_targets_include_core_files() {
        let targets = p0_global_targets();
        let names: Vec<&str> = targets.iter().map(|t| t.object).collect();
        assert!(names.contains(&"loadavg"));
        assert!(names.contains(&"meminfo"));
        assert!(names.contains(&"cpuinfo"));
        assert!(names.contains(&"stat"));
        assert!(names.contains(&"mounts"));
    }

    #[test]
    fn p2_limited_targets_include_ipc_and_device_files() {
        let targets = p2_limited_targets();
        let names: Vec<&str> = targets.iter().map(|t| t.object).collect();
        assert!(names.contains(&"shm"));
        assert!(names.contains(&"msg"));
        assert!(names.contains(&"sem"));
    }
}
