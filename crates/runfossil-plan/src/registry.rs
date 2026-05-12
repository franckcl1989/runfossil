#![forbid(unsafe_code)]
#![doc = "Coverage registry: the canonical list of all planned coverage units. Defines source coverage intent independently from per-capture scheduling."]

use runfossil_core::{CoverageDecision, ObjectKind, Priority, SourceSlug};

#[derive(Clone, Debug)]
pub(crate) struct CoverageUnit {
    pub id: String,
    pub source: SourceSlug,
    pub domain: String,
    pub object: String,
    pub coverage_decision: CoverageDecision,
    pub priority: Priority,
    #[allow(dead_code)]
    pub object_kind: ObjectKind,
    pub requires_root: bool,
}

impl CoverageUnit {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: impl Into<String>,
        source: SourceSlug,
        domain: impl Into<String>,
        object: impl Into<String>,
        coverage_decision: CoverageDecision,
        priority: Priority,
        object_kind: ObjectKind,
        requires_root: bool,
    ) -> Self {
        Self {
            id: id.into(),
            source,
            domain: domain.into(),
            object: object.into(),
            coverage_decision,
            priority,
            object_kind,
            requires_root,
        }
    }
}

#[allow(clippy::vec_init_then_push)]
pub(crate) fn coverage_registry() -> Vec<CoverageUnit> {
    let mut units = Vec::new();

    units.push(CoverageUnit::new(
        "proc.loadavg",
        SourceSlug::Proc,
        "system_overview",
        "loadavg",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.uptime",
        SourceSlug::Proc,
        "system_overview",
        "uptime",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.version",
        SourceSlug::Proc,
        "system_overview",
        "version",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.cmdline",
        SourceSlug::Proc,
        "system_overview",
        "cmdline",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.cpuinfo",
        SourceSlug::Proc,
        "system_overview",
        "cpuinfo",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.swaps",
        SourceSlug::Proc,
        "system_overview",
        "swaps",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.stat",
        SourceSlug::Proc,
        "cpu_scheduler",
        "stat",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.schedstat",
        SourceSlug::Proc,
        "cpu_scheduler",
        "schedstat",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.meminfo",
        SourceSlug::Proc,
        "memory_vm",
        "meminfo",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.vmstat",
        SourceSlug::Proc,
        "memory_vm",
        "vmstat",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.slabinfo",
        SourceSlug::Proc,
        "memory_vm",
        "slabinfo",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.zoneinfo",
        SourceSlug::Proc,
        "memory_vm",
        "zoneinfo",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.buddyinfo",
        SourceSlug::Proc,
        "memory_vm",
        "buddyinfo",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.pressure_cpu",
        SourceSlug::Proc,
        "psi_pressure",
        "pressure/cpu",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.pressure_memory",
        SourceSlug::Proc,
        "psi_pressure",
        "pressure/memory",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.pressure_io",
        SourceSlug::Proc,
        "psi_pressure",
        "pressure/io",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.interrupts",
        SourceSlug::Proc,
        "interrupt_softirq",
        "interrupts",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.softirqs",
        SourceSlug::Proc,
        "interrupt_softirq",
        "softirqs",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.diskstats",
        SourceSlug::Proc,
        "block",
        "diskstats",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.partitions",
        SourceSlug::Proc,
        "block",
        "partitions",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.mounts",
        SourceSlug::Proc,
        "mount_fs",
        "mounts",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.mountinfo",
        SourceSlug::Proc,
        "mount_fs",
        "self/mountinfo",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.filesystems",
        SourceSlug::Proc,
        "mount_fs",
        "filesystems",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_mountstats",
        SourceSlug::Proc,
        "mount_fs",
        "self/mountstats",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_dev",
        SourceSlug::Proc,
        "network",
        "net/dev",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_snmp",
        SourceSlug::Proc,
        "network",
        "net/snmp",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_netstat",
        SourceSlug::Proc,
        "network",
        "net/netstat",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_tcp",
        SourceSlug::Proc,
        "network",
        "net/tcp",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_tcp6",
        SourceSlug::Proc,
        "network",
        "net/tcp6",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_udp",
        SourceSlug::Proc,
        "network",
        "net/udp",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_udp6",
        SourceSlug::Proc,
        "network",
        "net/udp6",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_unix",
        SourceSlug::Proc,
        "network",
        "net/unix",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_raw",
        SourceSlug::Proc,
        "network",
        "net/raw",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_packet",
        SourceSlug::Proc,
        "network",
        "net/packet",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_arp",
        SourceSlug::Proc,
        "network",
        "net/arp",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_route",
        SourceSlug::Proc,
        "network",
        "net/route",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_ipv6_route",
        SourceSlug::Proc,
        "network",
        "net/ipv6_route",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_dev_mcast",
        SourceSlug::Proc,
        "network",
        "net/dev_mcast",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_igmp",
        SourceSlug::Proc,
        "network",
        "net/igmp",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_igmp6",
        SourceSlug::Proc,
        "network",
        "net/igmp6",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_netfilter",
        SourceSlug::Proc,
        "network",
        "net/netfilter",
        CoverageDecision::Limited,
        Priority::P3,
        ObjectKind::FileSet,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.modules",
        SourceSlug::Proc,
        "modules",
        "modules",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.locks",
        SourceSlug::Proc,
        "locks",
        "locks",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.sysvipc",
        SourceSlug::Proc,
        "ipc",
        "sysvipc",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::FileSet,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.iomem",
        SourceSlug::Proc,
        "hardware_map",
        "iomem",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.ioports",
        SourceSlug::Proc,
        "hardware_map",
        "ioports",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.devices",
        SourceSlug::Proc,
        "hardware_map",
        "devices",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.misc",
        SourceSlug::Proc,
        "hardware_map",
        "misc",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.cgroups",
        SourceSlug::Proc,
        "hardware_map",
        "cgroups",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.crypto",
        SourceSlug::Proc,
        "crypto",
        "crypto",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.key_users",
        SourceSlug::Proc,
        "crypto",
        "key-users",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_summary",
        SourceSlug::Proc,
        "process",
        "<pid>",
        CoverageDecision::Collect,
        Priority::P1,
        ObjectKind::FileSet,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_fd",
        SourceSlug::Proc,
        "process",
        "<pid>/fd",
        CoverageDecision::Collect,
        Priority::P1,
        ObjectKind::DirListing,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_ns",
        SourceSlug::Proc,
        "process",
        "<pid>/ns",
        CoverageDecision::Collect,
        Priority::P1,
        ObjectKind::Metadata,
        true,
    ));

    units.push(CoverageUnit::new(
        "sys.cgroup",
        SourceSlug::Sys,
        "cgroup",
        "fs/cgroup",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.pstore",
        SourceSlug::Sys,
        "pstore",
        "fs/pstore",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.power",
        SourceSlug::Sys,
        "power",
        "power",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::FileSet,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.class_net",
        SourceSlug::Sys,
        "network_device",
        "class/net",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.block",
        SourceSlug::Sys,
        "block_device",
        "block",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.devices_cpu",
        SourceSlug::Sys,
        "cpu_device",
        "devices/system/cpu",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.devices_node",
        SourceSlug::Sys,
        "numa",
        "devices/system/node",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.module",
        SourceSlug::Sys,
        "module",
        "module",
        CoverageDecision::Conditional,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.kernel",
        SourceSlug::Sys,
        "kernel",
        "kernel",
        CoverageDecision::Conditional,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.firmware",
        SourceSlug::Sys,
        "firmware",
        "firmware",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.hwmon",
        SourceSlug::Sys,
        "hwmon",
        "class/hwmon",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.thermal",
        SourceSlug::Sys,
        "thermal",
        "class/thermal",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.debugfs",
        SourceSlug::Sys,
        "debugfs",
        "kernel/debug",
        CoverageDecision::Limited,
        Priority::P4,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.tracefs",
        SourceSlug::Sys,
        "tracefs",
        "kernel/tracing",
        CoverageDecision::Limited,
        Priority::P4,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.bpffs",
        SourceSlug::Sys,
        "bpffs",
        "fs/bpf",
        CoverageDecision::Limited,
        Priority::P4,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.securityfs",
        SourceSlug::Sys,
        "securityfs",
        "kernel/security",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.class_drm",
        SourceSlug::Sys,
        "graphics",
        "class/drm",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.bus",
        SourceSlug::Sys,
        "bus",
        "bus",
        CoverageDecision::Limited,
        Priority::P4,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "sys.power_supply",
        SourceSlug::Sys,
        "power_supply",
        "class/power_supply",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "kmsg.bounded_window",
        SourceSlug::Kernel,
        "kmsg",
        "kmsg",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::EventWindow,
        false,
    ));

    units.push(CoverageUnit::new(
        "run.systemd",
        SourceSlug::Run,
        "systemd_runtime",
        "systemd",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "run.user",
        SourceSlug::Run,
        "user_runtime",
        "user",
        CoverageDecision::Limited,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "run.lock",
        SourceSlug::Run,
        "lock",
        "lock",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "run.pid_files",
        SourceSlug::Run,
        "daemon_pid",
        "*.pid",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::FileSet,
        false,
    ));

    units.push(CoverageUnit::new(
        "dev.metadata",
        SourceSlug::Dev,
        "device_node",
        "dev",
        CoverageDecision::Limited,
        Priority::P2,
        ObjectKind::Metadata,
        false,
    ));

    units.push(CoverageUnit::new(
        "netlink.link",
        SourceSlug::Netlink,
        "link",
        "link",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "netlink.address",
        SourceSlug::Netlink,
        "address",
        "address",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "netlink.route",
        SourceSlug::Netlink,
        "route",
        "route",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "netlink.neighbor",
        SourceSlug::Netlink,
        "neighbor",
        "neighbor",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "service.manager",
        SourceSlug::Service,
        "manager",
        "manager",
        CoverageDecision::Collect,
        Priority::P3,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "service.units",
        SourceSlug::Service,
        "units",
        "units",
        CoverageDecision::Collect,
        Priority::P3,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "container.process",
        SourceSlug::Container,
        "process",
        "process",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::FileSet,
        true,
    ));

    units.push(CoverageUnit::new(
        "container.resources",
        SourceSlug::Container,
        "resources",
        "resources",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        true,
    ));

    units.push(CoverageUnit::new(
        "container.namespace",
        SourceSlug::Container,
        "namespace",
        "namespace",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::Symlink,
        true,
    ));

    units.push(CoverageUnit::new(
        "container.cgroup",
        SourceSlug::Container,
        "cgroup",
        "cgroup",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        true,
    ));

    units.push(CoverageUnit::new(
        "sessions.logind",
        SourceSlug::Sessions,
        "logind",
        "logind",
        CoverageDecision::Collect,
        Priority::P3,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "security.selinux",
        SourceSlug::Security,
        "selinux",
        "selinux",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "hardware.sensors",
        SourceSlug::Hardware,
        "sensors",
        "sensors",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "crash.pstore",
        SourceSlug::Crash,
        "pstore",
        "pstore",
        CoverageDecision::Limited,
        Priority::P3,
        ObjectKind::MetadataOnly,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.process_environ",
        SourceSlug::Proc,
        "process",
        "<pid>/environ",
        CoverageDecision::Collect,
        Priority::P1,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_fdinfo",
        SourceSlug::Proc,
        "process",
        "<pid>/fdinfo",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::DirListing,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_maps",
        SourceSlug::Proc,
        "process",
        "<pid>/maps",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_smaps_rollup",
        SourceSlug::Proc,
        "process",
        "<pid>/smaps_rollup",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_numa_maps",
        SourceSlug::Proc,
        "process",
        "<pid>/numa_maps",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_stack",
        SourceSlug::Proc,
        "process",
        "<pid>/stack",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_wchan",
        SourceSlug::Proc,
        "process",
        "<pid>/wchan",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.process_task",
        SourceSlug::Proc,
        "process",
        "<pid>/task/<tid>",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::DirListing,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.pagetypeinfo",
        SourceSlug::Proc,
        "memory_vm",
        "pagetypeinfo",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.irq",
        SourceSlug::Proc,
        "interrupt_softirq",
        "irq",
        CoverageDecision::Limited,
        Priority::P2,
        ObjectKind::BoundedTree,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.keys",
        SourceSlug::Proc,
        "crypto",
        "keys",
        CoverageDecision::Limited,
        Priority::P4,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.timer_list",
        SourceSlug::Proc,
        "cpu_scheduler",
        "timer_list",
        CoverageDecision::Conditional,
        Priority::P4,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.kallsyms",
        SourceSlug::Proc,
        "modules",
        "kallsyms",
        CoverageDecision::Limited,
        Priority::P4,
        ObjectKind::File,
        true,
    ));

    units.push(CoverageUnit::new(
        "proc.net_route_all",
        SourceSlug::Proc,
        "network",
        "net/route",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.net_ipv6_route_all",
        SourceSlug::Proc,
        "network",
        "net/ipv6_route",
        CoverageDecision::Collect,
        Priority::P0,
        ObjectKind::File,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.sys_kernel",
        SourceSlug::Proc,
        "sysctl_kernel",
        "sys/kernel",
        CoverageDecision::Limited,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.sys_vm",
        SourceSlug::Proc,
        "sysctl_vm",
        "sys/vm",
        CoverageDecision::Limited,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.sys_fs",
        SourceSlug::Proc,
        "sysctl_fs",
        "sys/fs",
        CoverageDecision::Limited,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.sys_net",
        SourceSlug::Proc,
        "sysctl_net",
        "sys/net",
        CoverageDecision::Limited,
        Priority::P2,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.sys_user",
        SourceSlug::Proc,
        "sysctl_user",
        "sys/user",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "proc.process_security",
        SourceSlug::Proc,
        "process",
        "<pid>/security",
        CoverageDecision::Collect,
        Priority::P2,
        ObjectKind::FileSet,
        true,
    ));

    units.push(CoverageUnit::new(
        "service.manager_state",
        SourceSlug::Service,
        "manager",
        "state",
        CoverageDecision::Collect,
        Priority::P3,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "service.failed_units",
        SourceSlug::Service,
        "units",
        "failed",
        CoverageDecision::Collect,
        Priority::P3,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "sessions.users",
        SourceSlug::Sessions,
        "users",
        "users",
        CoverageDecision::Collect,
        Priority::P3,
        ObjectKind::NativeDump,
        false,
    ));

    units.push(CoverageUnit::new(
        "security.selinux_enforce",
        SourceSlug::Security,
        "selinux",
        "enforce",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "security.apparmor",
        SourceSlug::Security,
        "apparmor",
        "apparmor",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "scheduler.timers",
        SourceSlug::Scheduler,
        "timers",
        "timers",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "scheduler.cron",
        SourceSlug::Scheduler,
        "cron",
        "cron",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "scheduler.at",
        SourceSlug::Scheduler,
        "at_jobs",
        "at",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::BoundedTree,
        false,
    ));

    units.push(CoverageUnit::new(
        "crash.kdump",
        SourceSlug::Crash,
        "kdump",
        "kdump",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::Metadata,
        true,
    ));

    units.push(CoverageUnit::new(
        "crash.coredump",
        SourceSlug::Crash,
        "coredump",
        "coredump",
        CoverageDecision::Conditional,
        Priority::P3,
        ObjectKind::DirListing,
        false,
    ));

    units
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_registry_has_expected_sources() {
        let units = coverage_registry();
        let proc_count = units
            .iter()
            .filter(|u| u.source == SourceSlug::Proc)
            .count();
        let sys_count = units.iter().filter(|u| u.source == SourceSlug::Sys).count();
        assert!(proc_count > 0, "registry should contain proc units");
        assert!(sys_count > 0, "registry should contain sys units");
    }

    #[test]
    fn every_coverage_unit_has_non_empty_id() {
        for unit in &coverage_registry() {
            assert!(!unit.id.is_empty(), "coverage unit must have an id");
        }
    }

    #[test]
    fn registry_has_no_deferred_native_entries() {
        let units = coverage_registry();
        let deferred: Vec<_> = units
            .iter()
            .filter(|u| u.coverage_decision == CoverageDecision::DeferredNative)
            .collect();
        assert!(
            deferred.is_empty(),
            "registry should not contain deferred-native entries"
        );
    }

    #[test]
    fn registry_has_all_seven_source_families() {
        let units = coverage_registry();
        let sources: Vec<SourceSlug> = units.iter().map(|u| u.source).collect();
        let expected = [
            SourceSlug::Proc,
            SourceSlug::Sys,
            SourceSlug::Dev,
            SourceSlug::Kernel,
            SourceSlug::Netlink,
            SourceSlug::Service,
            SourceSlug::Container,
        ];
        for source in &expected {
            assert!(
                sources.contains(source),
                "registry should include source {source:?}"
            );
        }
    }

    #[test]
    fn coverage_units_have_non_empty_domain_and_object() {
        for unit in &coverage_registry() {
            assert!(
                !unit.domain.is_empty(),
                "coverage unit {} must have a domain",
                unit.id
            );
            assert!(
                !unit.object.is_empty(),
                "coverage unit {} must have an object",
                unit.id
            );
        }
    }

    #[test]
    fn registry_has_conditional_entries() {
        let units = coverage_registry();
        let conditional: Vec<_> = units
            .iter()
            .filter(|u| u.coverage_decision == CoverageDecision::Conditional)
            .collect();
        assert!(
            !conditional.is_empty(),
            "registry should include conditional entries"
        );
    }

    #[test]
    fn p0_units_have_collect_decision() {
        let units = coverage_registry();
        for unit in &units {
            if unit.priority == Priority::P0 {
                assert_eq!(
                    unit.coverage_decision,
                    CoverageDecision::Collect,
                    "P0 unit {} must have Collect decision",
                    unit.id
                );
            }
        }
    }

    #[test]
    fn all_coverage_decisions_present() {
        let units = coverage_registry();
        let mut decisions = Vec::new();
        for unit in &units {
            decisions.push(unit.coverage_decision);
        }
        assert!(decisions.iter().any(|d| *d == CoverageDecision::Collect));
        assert!(
            decisions
                .iter()
                .any(|d| *d == CoverageDecision::Conditional)
        );
        assert!(decisions.iter().any(|d| *d == CoverageDecision::Limited));
    }

    #[test]
    fn registry_unit_count_is_reasonable() {
        let units = coverage_registry();
        assert!(
            units.len() >= 30,
            "registry should have at least 30 coverage units, got {}",
            units.len()
        );
    }

    #[test]
    fn no_duplicate_unit_ids() {
        let units = coverage_registry();
        let mut ids = std::collections::HashSet::new();
        for unit in &units {
            assert!(
                ids.insert(&unit.id),
                "duplicate coverage unit id: {}",
                unit.id
            );
        }
    }
}
