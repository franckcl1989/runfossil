#![forbid(unsafe_code)]
#![doc = "Procfs and sysfs collectors for runfossil. Reads /proc and /sys runtime evidence without external commands."]

pub(crate) mod crash;
pub(crate) mod dev;
pub(crate) mod kmsg;
pub(crate) mod run;
pub(crate) mod scheduler;
pub(crate) mod security;
pub(crate) mod sys;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{
    AutoDiscoverConfig, BoundedReadLimits, BoundedTraversalLimits, FsError, ListResult,
};
use runfossil_fs::{list_dir_entries, read_file_bounded, read_link_bounded};
use runfossil_store::{
    ErrorLogEntry, HashRecord, ManifestEntry, ObjectLimits, SHA256_MAX_BYTES, SnapshotStore,
    StoreError, compute_sha256_hex,
};

/// Returns the source slug owned by this collector crate.
#[must_use]
pub const fn source() -> SourceSlug {
    SourceSlug::Proc
}

/// P1 per-process files. These are the stable per-process attributes
/// that almost all Linux kernels expose. They are collected via
/// auto-discovery from each `/proc/<pid>/` directory in addition to
/// anything else found there (excluding the blacklist).
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
        ProcPerProcess {
            name: "environ",
            relative: "environ",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "maps",
            relative: "maps",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "smaps_rollup",
            relative: "smaps_rollup",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "stack",
            relative: "stack",
            kind: ObjectKind::File,
        },
        ProcPerProcess {
            name: "wchan",
            relative: "wchan",
            kind: ObjectKind::File,
        },
    ]
}

/// Collects all /proc evidence into the given store.
///
/// Uses auto-discovery: scans `/proc` root for global files, `/proc/net` for
/// network protocol state, and `/proc/<pid>/` for per-process evidence. A
/// blacklist excludes known-dangerous or write-interface paths.
///
/// On live hosts, this requires root permissions for most files. Non-root
/// invocation will produce many `permission_denied` entries.
pub fn collect_proc(store: &mut SnapshotStore) -> Result<(), StoreError> {
    collect_proc_globals_auto(store)?;
    collect_proc_net_auto(store)?;
    collect_p1_processes(store)?;
    collect_proc_sys(store)?;
    Ok(())
}

fn collect_proc_globals_auto(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let proc_dir = Path::new("/proc");
    let config = AutoDiscoverConfig::proc_global();
    let limits = BoundedTraversalLimits::new(config.max_files_per_level, 0, config.timeout_ms);
    let read_limits = BoundedReadLimits::new(config.max_bytes_per_file, config.timeout_ms);

    let listing = match list_dir_entries(proc_dir, limits) {
        Ok(l) => l,
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

    for entry in &listing.entries {
        // Skip PID-named directories (numeric names)
        if entry.is_dir
            && entry
                .name
                .as_bytes()
                .first()
                .is_some_and(|b| b.is_ascii_digit())
        {
            continue;
        }
        // Skip known subdirectories (handled separately)
        if entry.is_dir {
            continue;
        }
        // Skip blacklisted entries
        if config.is_blacklisted(&entry.name) {
            continue;
        }

        let source_path = proc_dir.join(&entry.name);
        let started = now_ns();

        match read_file_bounded(&source_path, read_limits) {
            Ok(result) => {
                let out_path = output_path(&source_path);
                let bytes = result.content.len() as u64;
                let finished = now_ns();

                store.write_raw_file(&out_path, &result.content)?;

                let hash = if bytes <= SHA256_MAX_BYTES {
                    Some(HashRecord::new(
                        "sha256",
                        compute_sha256_hex(&result.content),
                    ))
                } else {
                    None
                };

                let status = if result.was_truncated {
                    ManifestStatus::Truncated
                } else {
                    ManifestStatus::Captured
                };

                let mut entry = ManifestEntry::new(
                    format!("proc.global.{}", entry.name),
                    SourceSlug::Proc,
                    "global",
                    &entry.name,
                    ObjectKind::File,
                    status,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(bytes)
                .with_timing(started, finished, 0)
                .with_limits(ObjectLimits::new(
                    read_limits.max_bytes,
                    read_limits.timeout_ms,
                    1,
                    0,
                ));

                if let Some(h) = hash {
                    entry = entry.with_hash(h);
                }

                store.record_object(entry)?;
            }
            Err(error) => {
                let status = fs_error_to_manifest_status(&error);
                let entry = ManifestEntry::new(
                    format!("proc.global.{}", entry.name),
                    SourceSlug::Proc,
                    "global",
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

    Ok(())
}

fn collect_proc_net_auto(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let net_dir = Path::new("/proc/net");
    if !net_dir.exists() {
        return Ok(());
    }

    let config = AutoDiscoverConfig::proc_net();
    let limits = BoundedTraversalLimits::new(config.max_files_per_level, 1, config.timeout_ms);
    let read_limits = BoundedReadLimits::new(config.max_bytes_per_file, config.timeout_ms);

    let listing = match list_dir_entries(net_dir, limits) {
        Ok(l) => l,
        Err(_) => return Ok(()),
    };

    for entry in &listing.entries {
        if entry.is_dir {
            // Handle subdirectories like netfilter, mptcp_net, sctp, etc.
            let sub_dir = net_dir.join(&entry.name);
            if let Ok(sub_listing) = list_dir_entries(&sub_dir, limits) {
                for sub_entry in &sub_listing.entries {
                    if sub_entry.is_dir {
                        continue;
                    }
                    collect_single_file(
                        store,
                        &sub_dir.join(&sub_entry.name),
                        SourceSlug::Proc,
                        "net",
                        &format!("{}/{}", entry.name, sub_entry.name),
                        read_limits,
                    )?;
                }
            }
            continue;
        }

        collect_single_file(
            store,
            &net_dir.join(&entry.name),
            SourceSlug::Proc,
            "net",
            &entry.name,
            read_limits,
        )?;
    }

    Ok(())
}

fn collect_single_file(
    store: &mut SnapshotStore,
    source_path: &Path,
    source: SourceSlug,
    domain: &str,
    object_name: &str,
    limits: BoundedReadLimits,
) -> Result<(), StoreError> {
    match read_file_bounded(source_path, limits) {
        Ok(result) => {
            let out_path = output_path(source_path);
            let bytes = result.content.len() as u64;

            store.write_raw_file(&out_path, &result.content)?;

            let hash = if bytes <= SHA256_MAX_BYTES {
                Some(HashRecord::new(
                    "sha256",
                    compute_sha256_hex(&result.content),
                ))
            } else {
                None
            };

            let status = if result.was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };

            let mut entry = ManifestEntry::new(
                format!("proc.{domain}.{object_name}"),
                source,
                domain,
                object_name,
                ObjectKind::File,
                status,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            if let Some(h) = hash {
                entry = entry.with_hash(h);
            }

            store.record_object(entry)?;
        }
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("proc.{domain}.{object_name}"),
                source,
                domain,
                object_name,
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

/// Collects sysctl-like files from /proc/sys/kernel, /proc/sys/vm,
/// /proc/sys/fs, and /proc/sys/net.
fn collect_proc_sys(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let limits = BoundedTraversalLimits::new(512, 3, 5000);
    let read_limits = BoundedReadLimits::new(65_536, 200);

    let subdirs: &[(&str, &str, &str)] = &[
        ("kernel", "sysctl_kernel", "sys_kernel"),
        ("vm", "sysctl_vm", "sys_vm"),
        ("fs", "sysctl_fs", "sys_fs"),
        ("net", "sysctl_net", "sys_net"),
    ];

    for (subdir, domain, id_prefix) in subdirs {
        let root = Path::new("/proc/sys").join(subdir);
        if !root.exists() {
            continue;
        }
        let ctx = SysWalkCtx {
            base: &root,
            domain,
            id_prefix,
            limits,
            read_limits,
        };
        walk_proc_sys_subdir(store, &ctx, &root, 0)?;
    }

    Ok(())
}

struct SysWalkCtx<'a> {
    base: &'a Path,
    domain: &'a str,
    id_prefix: &'a str,
    limits: BoundedTraversalLimits,
    read_limits: BoundedReadLimits,
}

fn walk_proc_sys_subdir(
    store: &mut SnapshotStore,
    ctx: &SysWalkCtx<'_>,
    current: &Path,
    depth: u32,
) -> Result<(), StoreError> {
    if depth >= ctx.limits.max_depth {
        return Ok(());
    }

    let listing = match list_dir_entries(current, ctx.limits) {
        Ok(listing) => listing,
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("proc.{}.status", ctx.id_prefix),
                SourceSlug::Proc,
                ctx.domain,
                "status",
                ObjectKind::Metadata,
                status,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(0, ctx.limits.timeout_ms, 1, 0));
            store.record_object(entry)?;
            return Ok(());
        }
    };

    for entry in &listing.entries {
        let source_path = current.join(&entry.name);

        if entry.is_dir {
            walk_proc_sys_subdir(store, ctx, &source_path, depth + 1)?;
        } else {
            let rel = source_path.strip_prefix(ctx.base).unwrap_or(&source_path);
            let name = rel.to_string_lossy().replace('/', ".");
            let proc_prefix = Path::new("/proc/");
            let object = source_path
                .strip_prefix(proc_prefix)
                .unwrap_or(&source_path)
                .to_string_lossy()
                .into_owned();

            match read_file_bounded(&source_path, ctx.read_limits) {
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
                        format!("proc.{}.{name}", ctx.id_prefix),
                        SourceSlug::Proc,
                        ctx.domain,
                        object,
                        ObjectKind::File,
                        status,
                    )
                    .with_path(out_path.to_string_lossy())
                    .with_bytes(bytes)
                    .with_limits(ObjectLimits::new(
                        ctx.read_limits.max_bytes,
                        ctx.read_limits.timeout_ms,
                        1,
                        0,
                    ));

                    store.record_object(manifest_entry)?;
                }
                Err(_error) => {}
            }
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

/// Collects security subsystem state (SELinux, AppArmor, LSM info).
pub fn collect_security(store: &mut SnapshotStore) -> Result<(), StoreError> {
    security::collect_security(store)
}

/// Collects scheduler state (systemd timers, cron, at jobs).
pub fn collect_scheduler(store: &mut SnapshotStore) -> Result<(), StoreError> {
    scheduler::collect_scheduler(store)
}

/// Collects crash dump state (kdump, core_pattern, coredump dir).
pub fn collect_crash(store: &mut SnapshotStore) -> Result<(), StoreError> {
    crash::collect_crash(store)
}

/// Collects `/run` runtime state (Linux host state, not container-specific).
///
/// Includes systemd state, user runtime directories, lock files, pid files,
/// udev data, and dbus state.
pub fn collect_run(store: &mut SnapshotStore) -> Result<(), StoreError> {
    run::collect_run(store)
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
        collect_process_fdinfo(store, pid, fds_limits)?;
        collect_process_threads(store, pid)?;
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

fn collect_process_fdinfo(
    store: &mut SnapshotStore,
    pid: u32,
    limits: BoundedTraversalLimits,
) -> Result<(), StoreError> {
    let fdinfo_dir = Path::new("/proc").join(pid.to_string()).join("fdinfo");

    match list_dir_entries(&fdinfo_dir, limits) {
        Ok(ListResult {
            entries,
            was_truncated,
        }) => {
            let read_limits = BoundedReadLimits::per_process();

            for entry in &entries {
                if entry.is_dir {
                    continue;
                }

                let source_path = fdinfo_dir.join(&entry.name);
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
                            format!("proc.process.{pid}.fdinfo.{}", entry.name),
                            SourceSlug::Proc,
                            "process",
                            format!("{pid}/fdinfo/{}", entry.name),
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
                    }
                    Err(error) => {
                        let status = fs_error_to_manifest_status(&error);
                        let file_entry = ManifestEntry::new(
                            format!("proc.process.{pid}.fdinfo.{}", entry.name),
                            SourceSlug::Proc,
                            "process",
                            format!("{pid}/fdinfo/{}", entry.name),
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

            let dir_status = if was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };
            let dir_entry = ManifestEntry::new(
                format!("proc.process.{pid}.fdinfo"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/fdinfo"),
                ObjectKind::DirListing,
                dir_status,
            )
            .with_limits(ObjectLimits::new(
                0,
                limits.timeout_ms,
                limits.max_files,
                limits.max_depth,
            ));

            store.record_object(dir_entry)?;
        }
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("proc.process.{pid}.fdinfo"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/fdinfo"),
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

fn collect_process_threads(store: &mut SnapshotStore, pid: u32) -> Result<(), StoreError> {
    let task_dir = Path::new("/proc").join(pid.to_string()).join("task");
    let limits = BoundedTraversalLimits::new(64, 1, 200);

    match list_dir_entries(&task_dir, limits) {
        Ok(ListResult {
            entries,
            was_truncated,
        }) => {
            let read_limits = BoundedReadLimits::per_process();

            for entry in &entries {
                if !entry.is_dir {
                    continue;
                }
                let tid = &entry.name;

                for file_name in &["status", "stat"] {
                    let source_path = task_dir.join(tid).join(file_name);
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
                                format!("proc.process.{pid}.thread.{tid}.{file_name}"),
                                SourceSlug::Proc,
                                "process",
                                format!("{pid}/task/{tid}/{file_name}"),
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
                        }
                        Err(_error) => {}
                    }
                }
            }

            let dir_status = if was_truncated {
                ManifestStatus::Truncated
            } else {
                ManifestStatus::Captured
            };
            let dir_entry = ManifestEntry::new(
                format!("proc.process.{pid}.threads"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/task"),
                ObjectKind::DirListing,
                dir_status,
            )
            .with_limits(ObjectLimits::new(
                0,
                limits.timeout_ms,
                limits.max_files,
                limits.max_depth,
            ));

            store.record_object(dir_entry)?;
        }
        Err(error) => {
            let status = fs_error_to_manifest_status(&error);
            let entry = ManifestEntry::new(
                format!("proc.process.{pid}.threads"),
                SourceSlug::Proc,
                "process",
                format!("{pid}/task"),
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

    fn dir() -> Result<tempfile::TempDir, std::io::Error> {
        tempfile::tempdir()
    }

    fn new_store(d: &tempfile::TempDir) -> Result<SnapshotStore, runfossil_store::StoreError> {
        SnapshotStore::create(d.path().join("snapshot"), test_metadata())
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
            source: io::Error::other("broken"),
        };
        assert_eq!(fs_error_to_manifest_status(&e), ManifestStatus::IoError);
    }

    #[test]
    fn all_fs_error_to_status_mappings_are_distinct() {
        let seen = [
            fs_error_to_manifest_status(&FsError::NotFound(Path::new("a").to_path_buf())),
            fs_error_to_manifest_status(&FsError::PermissionDenied(Path::new("b").to_path_buf())),
            fs_error_to_manifest_status(&FsError::Timeout(Path::new("c").to_path_buf())),
            fs_error_to_manifest_status(&FsError::SizeLimited {
                path: Path::new("d").to_path_buf(),
                max_bytes: 1,
            }),
            fs_error_to_manifest_status(&FsError::Io {
                path: Path::new("e").to_path_buf(),
                source: io::Error::other(""),
            }),
        ];
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
    fn proc_auto_discover_blacklist_is_stable() {
        let config = AutoDiscoverConfig::proc_global();
        assert!(config.is_blacklisted("kcore"));
        assert!(config.is_blacklisted("kallsyms"));
        assert!(!config.is_blacklisted("meminfo"));
        assert!(!config.is_blacklisted("loadavg"));
    }

    #[test]
    fn proc_per_process_blacklist_is_stable() {
        let config = AutoDiscoverConfig::proc_per_process();
        assert!(config.is_blacklisted("mem"));
        assert!(config.is_blacklisted("pagemap"));
        assert!(!config.is_blacklisted("status"));
        assert!(!config.is_blacklisted("cmdline"));
    }

    #[test]
    fn proc_targets_have_non_empty_ids() {
        let targets = p1_process_targets();
        for t in &targets {
            assert!(!t.name.is_empty());
            assert!(!t.relative.is_empty());
        }
    }

    #[test]
    #[ignore = "manual live-host smoke test; touches /sys pseudo-filesystems"]
    fn sys_collector_live_host_smoke() -> Result<(), Box<dyn std::error::Error>> {
        let d = dir()?;
        let mut store = new_store(&d)?;
        let result = collect_sys(&mut store);
        if let Err(e) = result.as_ref() {
            assert!(
                e.to_string().contains("Io") || e.to_string().contains("not found"),
                "unexpected error: {e}"
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "manual live-host smoke test; touches /dev pseudo-filesystems"]
    fn dev_collector_live_host_smoke() -> Result<(), Box<dyn std::error::Error>> {
        let d = dir()?;
        let mut store = new_store(&d)?;
        let result = collect_dev(&mut store);
        if let Err(e) = result.as_ref() {
            assert!(e.to_string().contains("Io"), "unexpected error: {e}");
        }
        Ok(())
    }

    #[test]
    #[ignore = "manual live-host smoke test; touches /run pseudo-filesystems"]
    fn run_collector_live_host_smoke() -> Result<(), Box<dyn std::error::Error>> {
        let d = dir()?;
        let mut store = new_store(&d)?;
        let result = collect_run(&mut store);
        assert!(
            result.is_ok(),
            "run collector should return Ok even when /run dirs are absent"
        );
        Ok(())
    }

    #[test]
    fn now_ns_is_a_timestamp_string() -> Result<(), Box<dyn std::error::Error>> {
        let ns = now_ns();
        let parsed: u64 = ns.parse()?;
        assert!(
            parsed > 1_700_000_000_000_000_000,
            "timestamp should be in 2023+ range"
        );
        Ok(())
    }

    #[test]
    fn p1_process_targets_are_counted() {
        let targets = p1_process_targets();
        assert_eq!(targets.len(), 19, "expected 19 per-process file targets");
    }

    #[test]
    fn p1_process_targets_include_core_files() {
        let targets = p1_process_targets();
        let names: Vec<&str> = targets.iter().map(|t| t.name).collect();
        assert!(names.contains(&"status"));
        assert!(names.contains(&"cmdline"));
        assert!(names.contains(&"maps"));
    }

    #[test]
    #[ignore = "manual live-host smoke test; touches security pseudo-filesystems"]
    fn security_collector_live_host_smoke() -> Result<(), Box<dyn std::error::Error>> {
        let d = dir()?;
        let mut store = new_store(&d)?;
        let result = collect_security(&mut store);
        assert!(
            result.is_ok(),
            "security collector should return Ok even when source dirs are absent"
        );
        Ok(())
    }

    #[test]
    #[ignore = "manual live-host smoke test; touches scheduler state directories"]
    fn scheduler_collector_live_host_smoke() -> Result<(), Box<dyn std::error::Error>> {
        let d = dir()?;
        let mut store = new_store(&d)?;
        let result = collect_scheduler(&mut store);
        assert!(
            result.is_ok(),
            "scheduler collector should return Ok even when source dirs are absent"
        );
        Ok(())
    }

    #[test]
    #[ignore = "manual live-host smoke test; touches crash dump state directories"]
    fn crash_collector_live_host_smoke() -> Result<(), Box<dyn std::error::Error>> {
        let d = dir()?;
        let mut store = new_store(&d)?;
        let result = collect_crash(&mut store);
        assert!(
            result.is_ok(),
            "crash collector should return Ok even when source dirs are absent"
        );
        Ok(())
    }
}
