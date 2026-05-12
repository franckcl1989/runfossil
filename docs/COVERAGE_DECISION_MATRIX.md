# Coverage Decision Matrix

## Purpose

This document is the authoritative collection decision matrix for `runfossil`.
It maps the complete Linux runtime source taxonomy to project collection
decisions with explicit auto-discovery strategies and blacklist rules.

## Decision Vocabulary

```text
collect-auto
  Auto-discover available objects by scanning the source root. Collect
  everything found except blacklisted entries. Bounds still apply.

collect
  Required target with explicit path or method. Used where auto-discovery
  is infeasible (D-Bus, netlink, kmsg).

conditional
  Valid target. Collect only when the source exists, native support is
  present, and planner budget allows.

limited
  Valid target, intentionally bounded by strict size, count, depth,
  metadata-only, or selected-object policy.

deferred-native
  Valid target, but not collected until a Rust-native implementation
  exists. External commands are not an acceptable substitute.

exclude
  Intentionally not collected: outside scope, unsafe, destructive,
  unbounded, application-layer, or not raw Linux runtime evidence.
```

## Priority Vocabulary

```text
P0  Core global runtime evidence — small, volatile, high-value.
P1  Process and immediate runtime summary evidence.
P2  Subsystem runtime evidence — devices, cgroups, filesystems.
P3  Conditional deep evidence — per-process details, D-Bus, logs.
P4  High-cost or high-risk evidence — debugfs, bus trees, large payloads.
NA  Not scheduled for collection.
```

## Collection Mode Vocabulary

```text
auto-discover    Scan root; collect all non-blacklisted entries.
raw-file         Single bounded file read.
raw-file-set     Set of related files.
dir-listing      Directory entry names + types.
symlink-targets  Symlink resolution targets.
metadata         File metadata (type, permissions, size, dev numbers).
bounded-tree     Depth/count-limited recursive tree walk.
bounded-window   Byte-limited event window read.
native-protocol  Native Rust protocol (D-Bus).
native-netlink   Native Rust netlink dump.
metadata-only    Existence + metadata; no content copy.
skip             Not collected.
```

## Blacklist Conventions

Each auto-discovery domain defines a **blacklist** of entries that must not be
collected. Blacklist entries fall into these categories:

- **write-interface** — reading may trigger a kernel action (sysrq-trigger,
  clear_refs, bind, unbind, probe, reset, trigger)
- **memory-dump** — reading returns raw memory contents (kcore, mem, pagemap,
  /dev/mem, /dev/kmem, /dev/port)
- **security-sensitive** — reading exposes kernel addresses or key material
  (kallsyms)
- **device-content** — reading device nodes may block, stream, or have side
  effects (watchdog, sg*, bsg*, cpu/*/msr)
- **application-layer** — belongs to application software, not Linux runtime
  (container engine sockets)

---

## 1. /proc — Global files

**Strategy**: Scan `/proc` root; skip PID-named directories; collect all
remaining regular files. Single files bounded at 1 MiB / 100ms.

**Blacklist**: `kcore`, `kallsyms`, `kpagecount`, `kpageflags`,
`kpagecgroup`, `sysrq-trigger`

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| System overview | `/proc/` non-PID files | collect-auto | P0 | auto-discover | Global host state; blacklist excludes memory/symbol dumps |
| CPU / scheduler | `/proc/stat`, `/proc/schedstat` | collect-auto | P0 | auto-discover | Discovered in scan |
| CPU / scheduler | `/proc/sched_debug` | conditional | P4 | raw-file | Large; collect under budget only |
| Memory / VM | `/proc/meminfo`, `/proc/vmstat`, etc. | collect-auto | P0 | auto-discover | Core memory evidence |
| Memory / VM | `/proc/slabinfo`, `/proc/zoneinfo`, `/proc/buddyinfo` | collect-auto | P2 | auto-discover | Kernel memory details |
| Memory / VM | `/proc/pagetypeinfo` | conditional | P3 | raw-file | Larger; collect under budget |
| PSI pressure | `/proc/pressure/cpu`, `memory`, `io`, `irq` | collect-auto | P0 | auto-discover | All PSI files auto-discovered |
| Interrupt / softirq | `/proc/interrupts`, `/proc/softirqs` | collect-auto | P0 | auto-discover | Core interrupt evidence |
| Interrupt / softirq | `/proc/irq/*` | collect-auto | P2 | bounded-tree | IRQ details with depth limits |
| Block | `/proc/diskstats`, `/proc/partitions` | collect-auto | P0 | auto-discover | Block I/O evidence |
| Block | `/proc/mdstat` | collect-auto | P2 | raw-file | Software RAID state |
| Network stack | `/proc/net/` all files | collect-auto | P0-P2 | auto-discover | Full /proc/net scan; all files are safe text |
| IPC | `/proc/sysvipc/*` | collect-auto | P2 | auto-discover | SysV IPC objects |
| File locks | `/proc/locks` | collect-auto | P2 | raw-file | Active file lock evidence |
| Mount / filesystem | `/proc/mounts`, `/proc/filesystems`, etc. | collect-auto | P0 | auto-discover | Mount table evidence |
| Mount / filesystem | `/proc/self/mountinfo`, `mountstats` | collect-auto | P0-P2 | auto-discover | Per-process mount view |
| Filesystem-specific | `/proc/fs/*` | collect-auto | P3 | auto-discover | Auto-discover all fs subdirs |
| Kernel modules | `/proc/modules` | collect-auto | P0 | auto-discover | Loaded module inventory |
| Hardware maps | `/proc/iomem`, `/proc/ioports`, etc. | collect-auto | P2 | auto-discover | Resource maps |
| Crypto / keys | `/proc/crypto`, `/proc/key-users` | collect-auto | P2 | auto-discover | Crypto state |
| Crypto / keys | `/proc/keys` | limited | P4 | raw-file | Permission-sensitive; bounded |
| Kernel symbols | `/proc/kallsyms` | **blacklist** | NA | skip | Security-sensitive |
| Kernel memory | `/proc/kcore` | **blacklist** | NA | skip | Kernel memory image |
| Page metadata | `/proc/kpage*` | **blacklist** | NA | skip | Large physical page dumps |

---

## 2. /proc — Per-process

**Strategy**: Scan each `/proc/<pid>/` directory; collect all regular files
except blacklisted entries. Symlinks recorded by target resolution.

**Blacklist** (per-process): `mem`, `pagemap`, `clear_refs`, `oom_adj`,
`coredump_filter`, `uid_map`, `gid_map`, `projid_map`, `setgroups`,
`reclaim`, `attr/*`

**Deprioritized** (tighter limits): `smaps`

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Process summary | `/proc/<pid>/` all safe files | collect-auto | P1 | auto-discover | Full per-process discovery |
| Process symlinks | `/proc/<pid>/cwd`, `root`, `exe` | collect-auto | P1 | auto-discover | Resolve symlink targets |
| File descriptors | `/proc/<pid>/fd/*` | collect-auto | P1 | dir-listing | FD inventory with symlink targets |
| FD info | `/proc/<pid>/fdinfo/*` | conditional | P3 | auto-discover | Valuable but scales with FD count |
| Namespaces | `/proc/<pid>/ns/*` | collect-auto | P1 | symlink-targets | Namespace relation evidence |
| Threads | `/proc/<pid>/task/*/` | conditional | P3 | auto-discover | Per-thread status; bounded on large hosts |
| Process net | `/proc/<pid>/net/*` | collect-auto | P2 | auto-discover | Per-process net namespace |
| Process mounts | `/proc/<pid>/mounts`, `mountinfo` | collect-auto | P2 | auto-discover | Per-process mount view |
| Memory maps | `/proc/<pid>/smaps` | limited | P4 | raw-file | Large; selected PIDs only |
| Memory maps | `/proc/<pid>/smaps_rollup` | collect-auto | P2 | raw-file | Aggregate, lower cost |
| Memory maps | `/proc/<pid>/maps` | collect-auto | P2 | raw-file | Memory layout |
| Process memory | `/proc/<pid>/mem` | **blacklist** | NA | skip | Process memory image |
| Page table | `/proc/<pid>/pagemap` | **blacklist** | NA | skip | Large page table dump |
| Write interfaces | `clear_refs`, `oom_adj`, `attr/*`, etc. | **blacklist** | NA | skip | Mutate kernel state |

---

## 3. /proc/sys — Sysctl

**Strategy**: Recursive bounded-depth scan from `/proc/sys/` root. All sysctl
files are safe read-only text. No blacklist needed.

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Sysctl | `/proc/sys/` full tree | collect-auto | P2 | bounded-tree | Depth ≤ 4, 512 files/level, 64 KiB/file |
| Kernel params | `/proc/sys/kernel/*` | collect-auto | P2 | bounded-tree | Included in tree walk |
| VM params | `/proc/sys/vm/*` | collect-auto | P2 | bounded-tree | Included in tree walk |
| FS params | `/proc/sys/fs/*` | collect-auto | P2 | bounded-tree | Included in tree walk |
| Net params | `/proc/sys/net/*` | collect-auto | P2 | bounded-tree | Included in tree walk |
| User params | `/proc/sys/user/*` | collect-auto | P3 | bounded-tree | Included in tree walk |
| Debug params | `/proc/sys/debug/*` | collect-auto | P3 | bounded-tree | Auto-discovered if present |
| Dev params | `/proc/sys/dev/*` | collect-auto | P3 | bounded-tree | Auto-discovered if present |
| ABI params | `/proc/sys/abi/*` | collect-auto | P3 | bounded-tree | Auto-discovered if present |

---

## 4. /sys — Device and kernel object model

**Strategy**: Bounded-depth recursive scan. For each device directory,
auto-discover all attribute files. Depth ≤ 4, 256 files/level, 1 MiB/file.

**Global blacklist** (write interfaces): `uevent`, `bind`, `unbind`,
`probe`, `reset`, `trigger`, `store`, `config`

**Additional blacklist** (power): `/sys/power/state`, `/sys/power/disk`
(may trigger suspend/hibernate)

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Network devices | `/sys/class/net/*/` | collect-auto | P2 | auto-discover | All interface attributes; blacklist write interfaces |
| Network devices | `/sys/devices/virtual/net/*/` | collect-auto | P2 | auto-discover | Virtual network devices |
| Block devices | `/sys/block/*/` | collect-auto | P2 | auto-discover | All disk attributes + queue params |
| Block devices | `/sys/block/zram*/` | collect-auto | P2 | auto-discover | Compressed RAM block devices |
| CPU devices | `/sys/devices/system/cpu/*/` | collect-auto | P2 | auto-discover | Topology, cache, freq, idle |
| CPU vulnerabilities | `/sys/devices/system/cpu/vulnerabilities/` | collect-auto | P0 | auto-discover | **Critical incident evidence** |
| CPU microcode | `/sys/devices/system/cpu/microcode/` | collect-auto | P2 | auto-discover | Microcode revision |
| CPU SMT | `/sys/devices/system/cpu/smt/` | collect-auto | P2 | auto-discover | SMT/HyperThreading controls |
| NUMA | `/sys/devices/system/node/*/` | collect-auto | P2 | auto-discover | NUMA topology and stats |
| Cgroup | `/sys/fs/cgroup/` | collect-auto | P2 | bounded-tree | Full cgroup tree, all controllers |
| Kernel modules | `/sys/module/*/` | collect-auto | P2 | auto-discover | Parameters, holders, sections |
| Kernel state | `/sys/kernel/` | collect-auto | P2 | auto-discover | General kernel runtime state |
| Kernel MM | `/sys/kernel/mm/*` | collect-auto | P2 | auto-discover | THP, KSM, hugepages, swap, compaction |
| Kernel livepatch | `/sys/kernel/livepatch/` | collect-auto | P2 | auto-discover | Hot-patch state |
| Kernel slab | `/sys/kernel/slab/` | collect-auto | P3 | auto-discover | SLAB merging info |
| Filesystem state | `/sys/fs/*` | collect-auto | P3 | bounded-tree | Per-fs-type stats (btrfs, xfs, ext4, fuse) |
| Firmware | `/sys/firmware/*` | collect-auto | P3 | bounded-tree | DMI, ACPI, EFI, devicetree |
| EFI variables | `/sys/firmware/efi/efivars/` | conditional | P4 | metadata | Sensitive; metadata-only |
| Power | `/sys/power/` safe files | collect-auto | P2 | auto-discover | Power state (excluding state/disk) |
| HWMON / thermal | `/sys/class/hwmon/*/`, `/sys/class/thermal/*/` | collect-auto | P2 | auto-discover | Sensors, temp, cooling devices |
| Power supply | `/sys/class/power_supply/*/` | collect-auto | P2 | auto-discover | Battery/PSU state |
| Graphics | `/sys/class/drm/*/`, `/sys/class/accel/*/` | collect-auto | P3 | auto-discover | GPU/accelerator state |
| RDMA | `/sys/class/infiniband*/` | collect-auto | P3 | auto-discover | RDMA/IB device state |
| NVMe | `/sys/class/nvme*/` | collect-auto | P3 | auto-discover | NVMe controller/subsystem |
| Storage classes | `/sys/class/scsi_*`, `/sys/class/ata_*`, `/sys/class/fc_*`, `/sys/class/sas_*`, `/sys/class/enclosure/*` | collect-auto | P3 | auto-discover | Storage device details |
| Misc devices | `/sys/class/watchdog`, `/sys/class/rtc`, `/sys/class/input`, `/sys/class/dmi`, `/sys/class/leds`, `/sys/class/backlight` | collect-auto | P3 | auto-discover | Miscellaneous device state |
| GPIO | `/sys/class/gpio/*/` | collect-auto | P3 | metadata | GPIO metadata only |
| TTY | `/sys/class/tty/*/`, `/sys/class/vtconsole/*/` | collect-auto | P3 | auto-discover | TTY device state |
| I2C/SPI | `/sys/class/i2c-*`, `/sys/class/spi_master/*` | collect-auto | P3 | metadata | Bus adapter metadata |
| Regulator | `/sys/class/regulator/*/` | collect-auto | P3 | auto-discover | Voltage regulator state |
| Other class | `/sys/class/extcon/*`, `/sys/class/devcoredump/*`, `/sys/class/devlink/*`, `/sys/class/pci_bus/*`, `/sys/class/mei/*`, `/sys/class/mic/*`, `/sys/class/uwb_rc/*`, `/sys/class/wmi_bus/*` | conditional | P3 | auto-discover | Auto-discovered if present |
| DebugFS | `/sys/kernel/debug/` | limited | P4 | bounded-tree | Selected safe subtrees only |
| TraceFS | `/sys/kernel/tracing/` | limited | P4 | bounded-tree | Read existing state; do not enable |
| BPF filesystem | `/sys/fs/bpf/` | limited | P4 | metadata | Pinned object metadata only |
| SecurityFS | `/sys/kernel/security/` | collect-auto | P3 | bounded-tree | LSM, IMA, EVM, lockdown |
| ConfigFS | `/sys/kernel/config/` | conditional | P3 | metadata | Dynamic config metadata |
| Pstore | `/sys/fs/pstore/` | collect-auto | P2 | bounded-tree | Crash evidence — high value |
| Virtualization | `/sys/hypervisor/`, `/sys/devices/virtual/` | collect-auto | P3 | bounded-tree | Hypervisor/VM evidence |
| KVM | `/sys/module/kvm*/parameters/` | collect-auto | P3 | auto-discover | KVM module state |
| IOMMU | `/sys/kernel/iommu_groups/`, `/sys/class/iommu/*/` | collect-auto | P3 | auto-discover | IOMMU topology |
| Bus/devices | `/sys/bus/*/devices/`, `/sys/devices/` | limited | P4 | bounded-tree | Very large; strict bounds |
| EDAC | `/sys/devices/system/edac/` | collect-auto | P2 | auto-discover | Memory error counters |
| MCE | `/sys/devices/system/machinecheck/` | collect-auto | P2 | auto-discover | Machine check exceptions |

---

## 5. /run

**Strategy**: Scan `/run` root for discrete files → collect all. Subdirectories
with bounded depth (3) and file count (64/level). Socket files → metadata only.

No blacklist needed for `/run` — all regular files are safe. Socket files,
FIFO files, and unusually large trees are bounded by the traversal limits.

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| systemd runtime | `/run/systemd/` | collect-auto | P3 | bounded-tree | Service manager runtime |
| User runtime | `/run/user/*/` | collect-auto | P3 | bounded-tree | Per-user runtime; depth-limited |
| Locks | `/run/lock/` | collect-auto | P2 | bounded-tree | Active lock evidence |
| PID files | `/run/*.pid` | collect-auto | P2 | auto-discover | Daemon PID evidence |
| udev runtime | `/run/udev/` | collect-auto | P3 | bounded-tree | Device runtime state |
| D-Bus runtime | `/run/dbus/` | collect-auto | P3 | metadata | Socket presence only |
| Resolver | `/run/systemd/resolve/` | collect-auto | P3 | bounded-tree | DNS resolver state |
| Login records | `/run/utmp` | collect-auto | P3 | raw-file | Login session records |
| Login failures | `/run/faillock/` | collect-auto | P3 | bounded-tree | Failed login records |
| User mounts | `/run/mount/utab` | collect-auto | P3 | raw-file | User mount table |
| NetworkManager | `/run/NetworkManager/` | conditional | P3 | bounded-tree | NM runtime state if present |
| chrony | `/run/chrony/` | conditional | P3 | bounded-tree | chrony runtime state if present |
| Application sockets | `/run/docker.sock`, `/run/containerd/`, `/run/crio/`, `/run/runc/`, `/run/podman/`, `/run/kata-containers/`, `/run/gvisor/` | **exclude** | NA | skip | Application-layer container engines; not Linux system runtime |

---

## 6. /dev

**Strategy**: Metadata-only — device node listings, symlink targets,
major/minor numbers, permissions, file types. Do not read device contents.

**Absolute blacklist**: `/dev/cpu/*/msr`, `/dev/mem`, `/dev/kmem`,
`/dev/port`, `/dev/watchdog*`, `/dev/sg*`, `/dev/bsg/*`

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Device nodes | `/dev/` top-level | collect-auto | P2 | metadata | Dir listing + metadata for all entries |
| Block symlinks | `/dev/block/`, `/dev/disk/`, `/dev/mapper/` | collect-auto | P2 | symlink-targets | Persistent device identifiers |
| Loop devices | `/dev/loop*` | collect-auto | P2 | metadata | Loop metadata only |
| FUSE device | `/dev/fuse` | collect-auto | P2 | metadata | Availability metadata |
| TTY/PTY | `/dev/tty*`, `/dev/pts/` | collect-auto | P3 | metadata | Terminal inventory; no stream reads |
| RNG | `/dev/random`, `/dev/urandom` | collect-auto | P2 | metadata | Metadata only |
| kmsg | `/dev/kmsg` | collect | P0 | bounded-window | Routed to kernel ring buffer collector |
| Pseudo devices | `/dev/null`, `/dev/zero`, `/dev/full`, `/dev/console`, `/dev/tty` | collect-auto | P2 | metadata | Metadata only |
| KVM | `/dev/kvm` | collect-auto | P3 | metadata | KVM availability metadata |
| VFIO | `/dev/vfio/*` | collect-auto | P3 | metadata | VFIO metadata only |
| vhost | `/dev/vhost*` | collect-auto | P3 | metadata | vhost metadata only |
| uinput/uhid | `/dev/uinput`, `/dev/uhid` | collect-auto | P3 | metadata | Input device metadata |
| MSR | `/dev/cpu/*/msr` | **blacklist** | NA | skip | Model-Specific Register access |
| Physical memory | `/dev/mem`, `/dev/kmem`, `/dev/port` | **blacklist** | NA | skip | Memory/port access |
| Watchdog | `/dev/watchdog*` | **blacklist** | NA | skip | Opening starts watchdog timer |
| SCSI generic | `/dev/sg*`, `/dev/bsg/*` | **blacklist** | NA | skip | Can send arbitrary SCSI commands |

---

## 7. Kernel Ring Buffer

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| kmsg window | `/dev/kmsg` non-blocking read | collect | P0 | bounded-window | 2 MiB, 500ms timeout, non-blocking |
| dmesg restrict | `/proc/sys/kernel/dmesg_restrict` | collect-auto | P0 | raw-file | Discovered in sysctl scan |

---

## 8. System Event Log Store

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| journald (run) | `/run/log/journal/*/` | conditional | P3 | bounded-window | Bounded file window; if present |
| journald (var) | `/var/log/journal/*/` | conditional | P3 | bounded-window | Bounded file window; if present |
| syslog | `/var/log/syslog` | conditional | P3 | bounded-window | If present; 256 KiB window |
| messages | `/var/log/messages` | conditional | P3 | bounded-window | If present |
| kern.log | `/var/log/kern.log` | conditional | P3 | bounded-window | If present |
| auth.log | `/var/log/auth.log` | conditional | P3 | bounded-window | If present |
| dmesg log | `/var/log/dmesg` | conditional | P3 | bounded-window | If present |
| lastlog | `/var/log/lastlog` | conditional | P3 | bounded-window | If present |

---

## 9. Service Manager

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Manager state | D-Bus Manager properties | collect | P3 | native-protocol | Version, features, architecture, cgroup |
| Unit list | D-Bus ListUnits | collect | P3 | native-protocol | All unit state summary |
| Unit details | D-Bus GetUnit + GetUnitFileState | collect | P3 | native-protocol | Per-unit state |
| Failed state | D-Bus ListUnitsFiltered | collect | P3 | native-protocol | Failed/degraded units |
| Jobs | D-Bus ListJobs | collect | P3 | native-protocol | Pending jobs |

---

## 10. Netlink

**Strategy**: Auto-probe known netlink families; attempt dump; unsupported
families marked explicitly.

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Link | `RTM_GETLINK` | collect | P2 | native-netlink | Interface inventory |
| Address | `RTM_GETADDR` | collect | P2 | native-netlink | IP address state |
| Route | `RTM_GETROUTE` | collect | P2 | native-netlink | Routing table |
| Neighbor | `RTM_GETNEIGH` | collect | P2 | native-netlink | ARP/NDP cache |
| Socket diagnostic | `NETLINK_SOCK_DIAG` | conditional | P3 | native-netlink | Socket state; can be large |
| Conntrack | `NETLINK_NETFILTER` CT | conditional | P3 | native-netlink | Connection tracking |
| Traffic control | `RTM_GETQDISC` etc. | conditional | P3 | native-netlink | Qdisc/class/filter state |
| XFRM | `NETLINK_XFRM` | conditional | P3 | native-netlink | IPsec state |
| nftables | `NETLINK_NETFILTER` nft | conditional | P3 | native-netlink | nftables ruleset |
| Crypto | `NETLINK_CRYPTO` | conditional | P3 | native-netlink | Kernel crypto users |
| RDMA | `NETLINK_RDMA` | conditional | P3 | native-netlink | RDMA device state |
| NVMe-oF | `NETLINK_NVME` | conditional | P3 | native-netlink | NVMe over Fabrics |
| TIPC | `NETLINK_TIPC` | conditional | P3 | native-netlink | Transparent IPC |
| SMC | `NETLINK_SMC` | conditional | P3 | native-netlink | SMC monitoring |
| Other families | Auto-probed | conditional | P3 | native-netlink | Mark unsupported if probe fails |

---

## 11. Security / Audit Subsystem

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Process security | `/proc/<pid>/status` caps, seccomp | collect-auto | P1 | auto-discover | Part of process scan |
| SELinux | `/sys/fs/selinux/` | collect-auto | P3 | bounded-tree | If SELinux is active |
| AppArmor | `/sys/kernel/security/apparmor/` | conditional | P3 | bounded-tree | If AppArmor is active |
| Audit login | `/proc/self/loginuid`, `sessionid` | collect-auto | P3 | raw-file | Audit session identity |
| IMA | `/sys/kernel/security/ima/` | conditional | P3 | bounded-tree | If IMA is active |
| EVM | `/sys/kernel/security/evm/` | conditional | P3 | bounded-tree | If EVM is active |
| Lockdown | `/sys/kernel/security/lockdown` | collect-auto | P3 | raw-file | Kernel lockdown level |
| Landlock | Process status detection | collect-auto | P3 | auto-discover | Detected in process scan |

---

## 12. User / Session Database

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| logind sessions | D-Bus ListSessions/ListUsers/GetSession | collect | P3 | native-protocol | Active session state |
| Login records | `/run/utmp` | collect-auto | P3 | raw-file | Traditional login records |

---

## 13. Scheduler / Job Runtime

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| systemd timers | D-Bus ListUnits (type=timer) | collect | P3 | native-protocol | Timer state via service manager |
| Cron | `/var/spool/cron/crontabs/`, `/var/spool/cron/atjobs/` | conditional | P3 | bounded-tree | If cron is present |
| Anacron | `/var/spool/anacron/` | conditional | P3 | bounded-tree | If anacron is present |
| At jobs | `/var/spool/at/` | conditional | P3 | bounded-tree | If at is present |
| fcron | `/var/spool/fcron/` | conditional | P3 | bounded-tree | If fcron is present |

---

## 14. Time Synchronization Subsystem

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| System clock | `/proc/uptime`, `/proc/stat` (btime) | collect-auto | P0 | auto-discovered | Essential capture context |
| Local time | `/etc/localtime`, `/etc/timezone` | collect-auto | P2 | symlink/raw | Timezone evidence |
| Adjtime | `/etc/adjtime` | conditional | P2 | raw-file | Clock drift/status |
| systemd-timesyncd | `/run/systemd/timesync/` | conditional | P3 | bounded-tree | If present |
| Clock source | `/sys/devices/system/clocksource/clocksource0/` | collect-auto | P2 | auto-discover | Active/available clock sources |
| PTP | `/sys/class/ptp/*/` | conditional | P3 | auto-discover | PTP hardware clocks |

---

## 15. Crash Dump Store

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| kdump | `/sys/kernel/kexec_crash_*` | collect-auto | P3 | auto-discover | Kdump state |
| Core pattern | `/proc/sys/kernel/core_pattern` | collect-auto | P3 | auto-discover | Part of sysctl scan |
| Core dump dir | `/var/lib/systemd/coredump/` | conditional | P3 | dir-listing | Metadata only; no large core files |
| Pstore | `/sys/fs/pstore/` | collect-auto | P2 | bounded-tree | Crash panic records |
| Crash dir | `/var/crash/` | conditional | P3 | metadata | Distribution crash dir; metadata only |

---

## 16. Hardware Management Interface

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Sensors | `/sys/class/hwmon/*/`, `/sys/class/thermal/*/` | collect-auto | P2 | auto-discover | Sensor/thermal state |
| Disk health | `/sys/block/*/device/` SCSI/SAS attrs | collect-auto | P3 | auto-discover | Disk device attributes |
| IDE legacy | `/proc/ide/*/` | conditional | P3 | metadata | Old IDE systems |
| MD RAID | `/proc/mdstat` | collect-auto | P2 | raw-file | Software RAID state |
| IPMI device | `/dev/ipmi*` | conditional | P3 | metadata | IPMI device presence |
| NVMe mgmt | NVMe admin commands | deferred-native | P4 | native-protocol | Requires safe NVMe interface |
| RAID mgmt | MegaRAID/PERC | deferred-native | P4 | native-protocol | Requires vendor interface |
| IPMI content | IPMI commands | deferred-native | P4 | native-protocol | Requires safe IPMI interface |
| GPU vendor | NVIDIA/AMD proprietary | deferred-native | P4 | native-protocol | Requires vendor interface |

---

## Explicit Exclusions

These are outside the core Linux runtime snapshot boundary:

| Exclusion | Decision | Rationale |
|---|---|---|
| Static system configuration files | exclude | Not runtime state |
| Application configuration and business data | exclude | Application scope |
| Source code and package manager databases | exclude | Static content |
| Application runtime sockets (Docker, containerd, etc.) | exclude | Application-layer, not Linux system runtime |
| Database internal runtime state | exclude | Database-specific scope |
| Language runtime internals (JVM, Go, Python, etc.) | exclude | Language-specific scope |
| Application debug endpoints and metrics | exclude | Application observability scope |
| Kubernetes API / orchestration state | exclude | Orchestration control-plane scope |
| Cloud provider control-plane APIs | exclude | Cloud control-plane scope |
| Full long-term historical logs | exclude | Unbounded; bounded windows only |
| External command output | exclude | Violates native Rust collection policy |
| Large core/vmcore/kernel memory payloads | exclude | Too large/risky; metadata only |
| `/proc/kcore`, `/proc/kallsyms` | exclude | Security-sensitive; blacklisted |
| `/proc/<pid>/mem`, `/proc/<pid>/pagemap` | exclude | Memory image/table dumps; blacklisted |
| `/dev/mem`, `/dev/kmem`, `/dev/port` | exclude | Memory/port access; blacklisted |
| `/dev/watchdog*` | exclude | Opens start watchdog; blacklisted |
| `/dev/sg*`, `/dev/bsg/*` | exclude | SCSI generic commands; blacklisted |
| `/dev/cpu/*/msr` | exclude | MSR access; blacklisted |
| `/proc/sysrq-trigger` | exclude | Write interface; blacklisted |
| Write interfaces (`bind`, `unbind`, `probe`, etc.) | exclude | Mutate kernel state; blacklisted |

---

## Review Rules

Changes to this matrix must also consider:

- The [Source Taxonomy](SOURCE_TAXONOMY.md), if source ownership changes.
- The [Capture Planning Strategy](CAPTURE_PLANNING_STRATEGY.md), if priority or
  budget behavior changes.
- The [Snapshot Specification](SNAPSHOT_SPECIFICATION.md), if new object statuses
  or storage modes are required.
- The [Implementation Plan](IMPLEMENTATION_PLAN.md), if delivery order changes.
- The [Requirements Traceability](REQUIREMENTS_TRACEABILITY.md), if a
  source moves into or out of accepted scope.

Before a design baseline is considered complete, every row must answer:

- What source or object family is covered?
- Is the object collected, conditional, limited, deferred-native, or excluded?
- What priority applies if it can run?
- What storage or collection mode applies?
- Why does the decision preserve forensic value without violating safety policy?
- What blacklist entries apply (for auto-discover domains)?
