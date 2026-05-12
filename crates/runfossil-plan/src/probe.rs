#![forbid(unsafe_code)]
#![doc = "Host probing for capture planning: reads /proc files, detects cgroup version, systemd, container sockets, and filesystem availability."]

use std::path::Path;

use runfossil_fs::{
    BoundedReadLimits, BoundedTraversalLimits, list_dir_entries, read_file_bounded,
};

use crate::{Pressure, PressureLevels};

const P_CPU: &str = "/proc/pressure/cpu";
const P_MEM: &str = "/proc/pressure/memory";
const P_IO: &str = "/proc/pressure/io";
const LOADAVG: &str = "/proc/loadavg";
const MEMINFO: &str = "/proc/meminfo";
const UPTIME: &str = "/proc/uptime";
const CPUINFO: &str = "/proc/cpuinfo";
const OSRELEASE: &str = "/proc/sys/kernel/osrelease";
const HOSTNAME: &str = "/proc/sys/kernel/hostname";
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";
const PIDS_MAX: &str = "/proc/sys/kernel/pid_max";
const SELF_STATUS: &str = "/proc/self/status";
const CGROUP_FS: &str = "/sys/fs/cgroup";
const PSTOREFS: &str = "/sys/fs/pstore";
const DEBUG_FS: &str = "/sys/kernel/debug";
const TRACE_FS: &str = "/sys/kernel/tracing";
const BPFFS: &str = "/sys/fs/bpf";
const SECURITYFS: &str = "/sys/kernel/security";
const SYSTEMD_PATH: &str = "/run/systemd";
const DOCKER_SOCK: &str = "/run/docker.sock";
const CONTAINERD_SOCK: &str = "/run/containerd/containerd.sock";
const CRIO_SOCK: &str = "/run/crio/crio.sock";

const S_V1: &str = "v1";
const S_V2: &str = "v2";
const S_NONE: &str = "none";

const CG_CONTROLLERS: &str = "cgroup.controllers";
const CG_SUBTREE: &str = "cgroup.subtree_control";

const PROBE_BYTES: u64 = 32_768;
const PROBE_TIMEOUT_MS: u64 = 5_000;
const PROBE_MAX_FILES: u64 = 16;

#[derive(Clone, Debug)]
pub struct HostProbe {
    pub root: bool,
    pub kernel_release: Option<String>,
    pub boot_id: Option<String>,
    pub hostname: Option<String>,
    pub cpu_count: u32,
    pub memory_total_kb: u64,
    pub memory_available_kb: Option<u64>,
    pub process_count: u32,
    pub pid_max: u32,
    pub load_average_1m: f64,
    pub load_average_5m: f64,
    pub load_average_15m: f64,
    pub uptime_seconds: f64,
    pub pressure: PressureLevels,
    pub cgroup_version: Option<String>,
    pub systemd_detected: bool,
    pub container_runtime_sockets: Vec<String>,
    pub pstore_available: bool,
    pub debugfs_available: bool,
    pub tracefs_available: bool,
    pub bpffs_available: bool,
    pub securityfs_available: bool,
}

impl HostProbe {
    pub fn empty() -> Self {
        Self {
            root: false,
            kernel_release: None,
            boot_id: None,
            hostname: None,
            cpu_count: 0,
            memory_total_kb: 0,
            memory_available_kb: None,
            process_count: 0,
            pid_max: 0,
            load_average_1m: 0.0,
            load_average_5m: 0.0,
            load_average_15m: 0.0,
            uptime_seconds: 0.0,
            pressure: PressureLevels::default(),
            cgroup_version: None,
            systemd_detected: false,
            container_runtime_sockets: Vec::new(),
            pstore_available: false,
            debugfs_available: false,
            tracefs_available: false,
            bpffs_available: false,
            securityfs_available: false,
        }
    }
}

impl Default for HostProbe {
    fn default() -> Self {
        Self::empty()
    }
}

/// Probes the host to gather facts needed for capture planning.
///
/// Reads `/proc` files, checks filesystem availability, detects cgroup version,
/// systemd presence, and container runtime sockets.
pub fn probe_host() -> HostProbe {
    let limits = BoundedReadLimits {
        max_bytes: PROBE_BYTES,
        timeout_ms: PROBE_TIMEOUT_MS,
    };

    let mut probe = HostProbe::empty();

    probe.root = check_is_root(limits);
    probe.kernel_release = read_line(OSRELEASE, limits);
    probe.boot_id = read_line(BOOT_ID, limits);
    probe.hostname = read_line(HOSTNAME, limits);

    let cpuinfo = read_file_bounded(Path::new(CPUINFO), limits);
    if let Ok(result) = cpuinfo {
        probe.cpu_count = count_lines_starting(&result.content, b"processor");
        if probe.cpu_count == 0 {
            probe.cpu_count = 1;
        }
    } else {
        probe.cpu_count = 1;
    }

    let meminfo = read_file_bounded(Path::new(MEMINFO), limits);
    if let Ok(result) = meminfo {
        probe.memory_total_kb = parse_kv_u64(&result.content, b"MemTotal:");
        probe.memory_available_kb = parse_kv_optional_u64(&result.content, b"MemAvailable:");
    }

    let loadavg = read_file_bounded(Path::new(LOADAVG), limits);
    if let Ok(result) = loadavg {
        let text = String::from_utf8_lossy(&result.content);
        if let Some(load) = parse_loadavg(&text) {
            probe.load_average_1m = load.0;
            probe.load_average_5m = load.1;
            probe.load_average_15m = load.2;
            probe.process_count = load.3;
            probe.pid_max = 32_768;
        }
    }

    let pid_max = read_file_bounded(Path::new(PIDS_MAX), limits);
    if let Ok(result) = pid_max {
        let text = String::from_utf8_lossy(&result.content);
        if let Ok(n) = text.trim().parse::<u32>() {
            probe.pid_max = n;
        }
    }

    let uptime = read_file_bounded(Path::new(UPTIME), limits);
    if let Ok(result) = uptime {
        let text = String::from_utf8_lossy(&result.content);
        if let Some(parts) = text.split_whitespace().next() {
            probe.uptime_seconds = parts.parse().unwrap_or(0.0);
        }
    }

    probe.pressure = read_pressure(limits);

    probe.cgroup_version = detect_cgroup_version_inner(limits);

    probe.systemd_detected = Path::new(SYSTEMD_PATH).exists();
    probe.pstore_available = Path::new(PSTOREFS).is_dir();
    probe.debugfs_available = Path::new(DEBUG_FS).is_dir();
    probe.tracefs_available = Path::new(TRACE_FS).is_dir();
    probe.bpffs_available = Path::new(BPFFS).is_dir();
    probe.securityfs_available = Path::new(SECURITYFS).is_dir();

    let mut sockets = Vec::new();
    for sock in &[DOCKER_SOCK, CONTAINERD_SOCK, CRIO_SOCK] {
        if Path::new(sock).exists() {
            sockets.push((*sock).to_string());
        }
    }
    probe.container_runtime_sockets = sockets;

    probe
}

fn check_is_root(limits: BoundedReadLimits) -> bool {
    let content = match read_file_bounded(Path::new(SELF_STATUS), limits) {
        Ok(r) => r.content,
        Err(_) => return false,
    };
    for line in content.split(|b| *b == b'\n') {
        if line.starts_with(b"Uid:") {
            let text = std::str::from_utf8(line).unwrap_or("");
            let mut fields = text.split_whitespace();
            fields.next(); // "Uid:"
            let _ = fields.next(); // real UID
            if let Some(effective) = fields.next() {
                return effective == "0";
            }
        }
    }
    false
}

fn read_line(path: &str, limits: BoundedReadLimits) -> Option<String> {
    let result = read_file_bounded(Path::new(path), limits).ok()?;
    let text = String::from_utf8_lossy(&result.content);
    let line = text.lines().next()?.trim().to_string();
    if line.is_empty() { None } else { Some(line) }
}

fn read_pressure(limits: BoundedReadLimits) -> PressureLevels {
    let cpu = parse_psi(P_CPU, limits);
    let memory = parse_psi(P_MEM, limits);
    let io = parse_psi(P_IO, limits);
    PressureLevels::new(cpu, memory, io)
}

fn parse_psi(path: &str, limits: BoundedReadLimits) -> Pressure {
    let content = match read_file_bounded(Path::new(path), limits) {
        Ok(r) => r.content,
        Err(_) => return Pressure::Low,
    };
    let text = String::from_utf8_lossy(&content);

    for line in text.lines() {
        if line.starts_with("some ") {
            for part in line.split_whitespace() {
                if let Some(val_str) = part.strip_prefix("avg10=")
                    && let Ok(val) = val_str.parse::<f64>()
                {
                    return classify_pressure(val);
                }
            }
        }
        if line.starts_with("full ") {
            for part in line.split_whitespace() {
                if let Some(val_str) = part.strip_prefix("avg10=")
                    && let Ok(val) = val_str.parse::<f64>()
                {
                    let full = classify_pressure(val);
                    if full_as_primary(full) {
                        return full;
                    }
                }
            }
        }
    }

    Pressure::Low
}

fn classify_pressure(value: f64) -> Pressure {
    if value > 25.0 {
        Pressure::Critical
    } else if value > 10.0 {
        Pressure::High
    } else if value > 2.0 {
        Pressure::Moderate
    } else {
        Pressure::Low
    }
}

fn full_as_primary(_pressure: Pressure) -> bool {
    false
}

fn count_lines_starting(content: &[u8], prefix: &[u8]) -> u32 {
    let mut count = 0u32;
    for line in content.split(|b| *b == b'\n') {
        if line.starts_with(prefix) {
            count += 1;
        }
    }
    count
}

fn parse_kv_u64(content: &[u8], key: &[u8]) -> u64 {
    for line in content.split(|b| *b == b'\n') {
        if let Some(val) = line.strip_prefix(key) {
            let s = std::str::from_utf8(val).unwrap_or("");
            let s = s.trim();
            if let Some(n) = s.split_whitespace().next()
                && let Ok(v) = n.parse::<u64>()
            {
                return v;
            }
        }
    }
    0
}

fn parse_kv_optional_u64(content: &[u8], key: &[u8]) -> Option<u64> {
    let v = parse_kv_u64(content, key);
    if v == 0 { None } else { Some(v) }
}

fn parse_loadavg(text: &str) -> Option<(f64, f64, f64, u32)> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let one = parts[0].parse::<f64>().ok()?;
    let five = parts[1].parse::<f64>().ok()?;
    let fifteen = parts[2].parse::<f64>().ok()?;
    let procs: Vec<&str> = parts[3].split('/').collect();
    if procs.len() < 2 {
        return None;
    }
    let running: u32 = procs[0].parse().ok()?;
    Some((one, five, fifteen, running))
}

fn detect_cgroup_version_inner(_limits: BoundedReadLimits) -> Option<String> {
    let cgroup_root = Path::new(CGROUP_FS);
    if !cgroup_root.is_dir() {
        return Some(S_NONE.to_string());
    }

    let ctrl_file = cgroup_root.join(CG_CONTROLLERS);
    let subtree_file = cgroup_root.join(CG_SUBTREE);

    let has_ctrl = ctrl_file.exists();
    let has_subtree = subtree_file.exists();

    if has_ctrl || has_subtree {
        if has_ctrl && has_subtree {
            return Some(S_V2.to_string());
        }
        let attrs = list_dir_entries(
            cgroup_root,
            BoundedTraversalLimits {
                max_files: PROBE_MAX_FILES,
                max_depth: 1,
                timeout_ms: PROBE_TIMEOUT_MS,
            },
        );

        if let Ok(list) = attrs {
            let v1_markers = ["cpuset", "memory", "cpu", "blkio", "devices", "freezer"];
            let mut has_v1 = false;
            for entry in &list.entries {
                if v1_markers.contains(&entry.name.as_str()) {
                    has_v1 = true;
                    break;
                }
            }
            if has_v1 {
                if has_ctrl || has_subtree {
                    return Some("hybrid".to_string());
                }
                return Some(S_V1.to_string());
            }
        }

        return Some(S_V2.to_string());
    }

    let attrs = list_dir_entries(
        cgroup_root,
        BoundedTraversalLimits {
            max_files: PROBE_MAX_FILES,
            max_depth: 1,
            timeout_ms: PROBE_TIMEOUT_MS,
        },
    );

    if let Ok(list) = attrs {
        let v1_markers = ["cpuset", "memory", "cpu", "blkio", "devices", "freezer"];
        let mut has_v1 = false;
        for entry in &list.entries {
            if v1_markers.contains(&entry.name.as_str()) {
                has_v1 = true;
                break;
            }
        }
        if has_v1 {
            return Some(S_V1.to_string());
        }
    }

    Some("unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_pressure_maps_to_expected_levels() {
        assert_eq!(classify_pressure(0.5), Pressure::Low);
        assert_eq!(classify_pressure(2.0), Pressure::Low);
        assert_eq!(classify_pressure(2.1), Pressure::Moderate);
        assert_eq!(classify_pressure(10.0), Pressure::Moderate);
        assert_eq!(classify_pressure(10.1), Pressure::High);
        assert_eq!(classify_pressure(25.0), Pressure::High);
        assert_eq!(classify_pressure(25.1), Pressure::Critical);
    }

    #[test]
    fn parse_loadavg_maps_to_expected_tuples() -> Result<(), &'static str> {
        let result = parse_loadavg("1.23 2.34 3.45 7/890 12345");
        let (one, five, fifteen, running) = result.ok_or("loadavg should parse")?;
        assert!((one - 1.23).abs() < f64::EPSILON);
        assert!((five - 2.34).abs() < f64::EPSILON);
        assert!((fifteen - 3.45).abs() < f64::EPSILON);
        assert_eq!(running, 7);
        Ok(())
    }

    #[test]
    fn host_probe_empty_has_no_tasks() {
        let probe = HostProbe::empty();
        assert!(!probe.root);
        assert_eq!(probe.cpu_count, 0);
        assert_eq!(probe.pressure, PressureLevels::default());
    }
}
