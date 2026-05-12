# Coverage Decision Matrix

## Purpose

This document is the authoritative collection decision matrix for `runfossil`.
It maps the complete Linux runtime source taxonomy to project collection
decisions.

The Source Taxonomy defines what exists. This matrix defines what `runfossil`
intends to collect, what it intentionally limits, what is conditional, what is
deferred until a native implementation exists, and what must not be collected.

## Completeness Rule

Each row is a coverage unit. A coverage unit may be a single file, a directory
tree, a symlink family, a native protocol response, or an event-window domain.

When a row names a source path or domain from the taxonomy, that row accounts for
all raw snapshot objects that belong to that path, object family, or domain. For
example, the `/proc/stat` row covers CPU counters, context switch counters, boot
time, fork counters, running process counters, and blocked process counters.

This keeps the matrix complete without duplicating every field-level description
from the taxonomy. `runfossil` preserves raw evidence; it does not need separate
collection decisions for each field inside a raw file.

Completeness is evaluated by assignment:

- Every L1 source family in the Source Taxonomy must have a section here.
- Every L2 domain in the Source Taxonomy must map to at least one row here.
- Every raw object family must have exactly one primary coverage unit, unless it
  is explicitly excluded.
- Overlapping rows are allowed only when one row is a fallback or supplement and
  the rationale says so.
- A new source, domain, or object family is incomplete until it has a decision,
  priority, mode, and rationale.

## Decision Vocabulary

```text
collect
  Required target. Capture when the source exists.

conditional
  Valid target. Capture only when the source exists, native support is present,
  and planner budget allows it.

limited
  Valid target, but intentionally bounded by size, count, depth, metadata-only,
  event-window, or selected-object policy.

deferred-native
  Valid target, but not collected until a Rust-native implementation exists.
  External commands are not an acceptable substitute.

exclude
  Intentionally not collected because it is outside scope, unsafe, destructive,
  unbounded, or not raw Linux runtime evidence.
```

## Priority Vocabulary

```text
P0  Core global runtime evidence.
P1  Process and immediate runtime summary evidence.
P2  Subsystem runtime evidence.
P3  Conditional deep evidence.
P4  High-cost or high-risk evidence.
NA  Not scheduled for collection.
```

## Collection Mode Vocabulary

```text
raw-file
raw-file-set
dir-listing
symlink-targets
metadata
bounded-tree
bounded-window
native-protocol
native-netlink
native-socket
metadata-only
skip
```

## Vocabulary Relationship

This matrix defines static coverage decisions. It does not define final runtime
outcomes. Runtime artifacts use two additional vocabularies:

- `plan.json` records a per-host plan decision such as `scheduled`, `limited`,
  `skipped_by_policy`, `unsupported`, or `not_present`.
- `manifest.json` records final object status such as `captured`, `vanished`,
  `not_found`, `permission_denied`, `timeout`, `size_limited`, `truncated`,
  `skipped_by_policy`, `unsupported`, or `io_error`.

The authoritative mapping between coverage decisions, plan decisions, and
manifest statuses is maintained in the
[Snapshot Specification](SNAPSHOT_SPECIFICATION.md).

## /proc

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| System overview | `/proc/loadavg` | collect | P0 | raw-file | Small volatile host pressure summary. |
| System overview | `/proc/uptime` | collect | P0 | raw-file | Small boot/runtime context. |
| System overview | `/proc/version` | collect | P0 | raw-file | Kernel build identity. |
| System overview | `/proc/cmdline` | collect | P0 | raw-file | Runtime boot parameters affect diagnosis. |
| System overview | `/proc/cpuinfo` | collect | P0 | raw-file | CPU topology and feature baseline. |
| System overview | `/proc/swaps` | collect | P0 | raw-file | Swap state is important for memory incidents. |
| CPU and scheduler | `/proc/stat` | collect | P0 | raw-file | Core CPU and scheduler counters. |
| CPU and scheduler | `/proc/schedstat` | collect | P0 | raw-file | Scheduler counters, low cost when present. |
| Memory and VM | `/proc/meminfo` | collect | P0 | raw-file | Core memory evidence. |
| Memory and VM | `/proc/vmstat` | collect | P0 | raw-file | VM activity, reclaim, swap, compaction, OOM counters. |
| Memory and VM | `/proc/slabinfo` | collect | P2 | raw-file | Kernel object memory state; usually bounded. |
| Memory and VM | `/proc/zoneinfo` | collect | P2 | raw-file | Memory zone state for pressure analysis. |
| Memory and VM | `/proc/buddyinfo` | collect | P2 | raw-file | Fragmentation evidence, low cost. |
| Memory and VM | `/proc/pagetypeinfo` | conditional | P3 | raw-file | Useful but can be larger; collect under budget. |
| PSI pressure | `/proc/pressure/cpu` | collect | P0 | raw-file | Direct runtime pressure signal. |
| PSI pressure | `/proc/pressure/memory` | collect | P0 | raw-file | Direct runtime pressure signal. |
| PSI pressure | `/proc/pressure/io` | collect | P0 | raw-file | Direct runtime pressure signal. |
| Interrupt and softirq | `/proc/interrupts` | collect | P0 | raw-file | Interrupt imbalance and device evidence. |
| Interrupt and softirq | `/proc/softirqs` | collect | P0 | raw-file | Softirq runtime evidence. |
| Interrupt and softirq | `/proc/irq` | limited | P2 | bounded-tree | Useful IRQ details, but traverse with depth and file limits. |
| Block statistics | `/proc/diskstats` | collect | P0 | raw-file | Core block I/O evidence. |
| Block statistics | `/proc/partitions` | collect | P0 | raw-file | Block device inventory. |
| Network stack | `/proc/net/tcp`, `/proc/net/tcp6` | collect | P0 | raw-file-set | Socket evidence, low cost compared with live command output. |
| Network stack | `/proc/net/udp`, `/proc/net/udp6` | collect | P0 | raw-file-set | Socket evidence. |
| Network stack | `/proc/net/unix`, `/proc/net/raw`, `/proc/net/packet` | collect | P0 | raw-file-set | Local socket and packet socket evidence. |
| Network stack | `/proc/net/dev` | collect | P0 | raw-file | Interface counters. |
| Network stack | `/proc/net/snmp`, `/proc/net/netstat` | collect | P0 | raw-file-set | Protocol counters. |
| Network stack | `/proc/net/arp`, `/proc/net/route`, `/proc/net/ipv6_route` | collect | P0 | raw-file-set | Route and neighbor raw fallback evidence. |
| Network stack | `/proc/net/dev_mcast`, `/proc/net/igmp`, `/proc/net/igmp6` | conditional | P2 | raw-file-set | Multicast state, low value on hosts without multicast use. |
| Network stack | `/proc/net/netfilter` | limited | P3 | bounded-tree | Netfilter runtime state can vary; use bounds. |
| IPC | `/proc/sysvipc/shm`, `/proc/sysvipc/msg`, `/proc/sysvipc/sem` | collect | P2 | raw-file-set | IPC objects are volatile and low cost. |
| File locks | `/proc/locks` | collect | P2 | raw-file | File-lock evidence is volatile. |
| Mount and filesystem view | `/proc/mounts` | collect | P0 | raw-file | Mount table evidence. |
| Mount and filesystem view | `/proc/self/mountinfo` | collect | P0 | raw-file | Mount namespace view. |
| Mount and filesystem view | `/proc/self/mountstats` | conditional | P2 | raw-file | Useful but may be large on NFS-heavy systems. |
| Mount and filesystem view | `/proc/filesystems` | collect | P0 | raw-file | Registered filesystem inventory. |
| Kernel parameters | `/proc/sys/kernel`, `/proc/sys/vm`, `/proc/sys/fs` | limited | P2 | bounded-tree | Runtime parameters are useful but should be bounded. |
| Kernel parameters | `/proc/sys/net`, `/proc/sys/user`, `/proc/sys/debug` | limited | P2 | bounded-tree | Runtime parameters are useful but should be bounded. |
| Kernel modules | `/proc/modules` | collect | P0 | raw-file | Loaded module evidence. |
| Kernel resource and hardware map | `/proc/iomem`, `/proc/ioports` | collect | P2 | raw-file-set | Hardware resource maps. |
| Kernel resource and hardware map | `/proc/devices`, `/proc/misc`, `/proc/cgroups` | collect | P2 | raw-file-set | Kernel device and cgroup controller inventory. |
| Kernel resource and hardware map | `/proc/acpi`, `/proc/scsi` | conditional | P3 | bounded-tree | Present only on some systems; bounded traversal. |
| Crypto, key, and timer | `/proc/crypto`, `/proc/key-users` | collect | P2 | raw-file-set | Runtime crypto and key user state. |
| Crypto, key, and timer | `/proc/keys` | limited | P4 | raw-file | Permission and sensitivity concerns; capture only if policy allows. |
| Crypto, key, and timer | `/proc/timer_list` | conditional | P4 | raw-file | Often restricted and potentially large. |
| Crypto, key, and timer | `/proc/kallsyms` | limited | P4 | raw-file | Sensitive and policy-restricted; bounded if captured. |
| Crypto, key, and timer | `/proc/kcore` | limited | P4 | metadata-only | Do not copy kernel core image; record metadata only. |
| Process and thread | `/proc/<pid>/status`, `stat`, `statm`, `cmdline`, `comm` | collect | P1 | raw-file-set | Core process summary. |
| Process and thread | `/proc/<pid>/environ` | limited | P3 | raw-file | Sensitive; bounded and may be skipped by policy. |
| Process and thread | `/proc/<pid>/cwd`, `root`, `exe` | collect | P1 | symlink-targets | Process relation evidence, low cost. |
| Process and thread | `/proc/<pid>/limits`, `io`, `sched`, `schedstat`, `cgroup` | collect | P1 | raw-file-set | Resource and scheduler evidence. |
| Process and thread | `/proc/<pid>/oom_score`, `oom_score_adj` | collect | P1 | raw-file-set | OOM diagnosis evidence. |
| Process and thread | `/proc/<pid>/ns` | collect | P1 | symlink-targets | Namespace relation evidence. |
| Process and thread | `/proc/<pid>/fd` | collect | P1 | dir-listing | FD inventory without reading targets as content. |
| Process and thread | `/proc/<pid>/fdinfo` | conditional | P3 | raw-file-set | Valuable but can scale with FD count. |
| Process and thread | `/proc/<pid>/maps`, `smaps_rollup`, `numa_maps` | conditional | P3 | raw-file-set | Memory mapping evidence, bounded by process count and pressure. |
| Process and thread | `/proc/<pid>/smaps` | limited | P4 | raw-file | High-cost per-process data; selected PIDs only. |
| Process and thread | `/proc/<pid>/pagemap` | limited | P4 | metadata-only | Sensitive and large; do not copy by default. |
| Process and thread | `/proc/<pid>/stack`, `wchan` | conditional | P3 | raw-file-set | Useful for blocked tasks; permission and kernel config dependent. |
| Process and thread | `/proc/<pid>/task` | conditional | P3 | dir-listing | Thread inventory; bounded on high-thread systems. |
| Process and thread | `/proc/<pid>/task/<tid>/status`, `stat`, `sched`, `schedstat`, `wchan`, `stack` | conditional | P3 | raw-file-set | Thread evidence; deepen under scale and incident signals. |

## /sys

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Network device | `/sys/class/net` | collect | P2 | bounded-tree | Interface attributes, statistics, queues, and device links. |
| Block device | `/sys/block` | collect | P2 | bounded-tree | Block attributes, queues, stats, partitions, mapper relations. |
| CPU device | `/sys/devices/system/cpu` | collect | P2 | bounded-tree | CPU online, topology, frequency, idle, vulnerability state. |
| NUMA | `/sys/devices/system/node` | collect | P2 | bounded-tree | NUMA node, memory, distance, hugepage evidence. |
| Cgroup | `/sys/fs/cgroup` | collect | P2 | bounded-tree | Cgroup hierarchy and controller state, bounded by scale. |
| Kernel module | `/sys/module` | conditional | P2 | bounded-tree | Module parameters and holders; bound traversal. |
| Kernel state | `/sys/kernel` | conditional | P2 | bounded-tree | Kernel state, livepatch, profiling, security, MM state. |
| DebugFS | `/sys/kernel/debug` | limited | P4 | bounded-tree | Policy-sensitive and potentially large; collect selected safe subtrees only. |
| TraceFS | `/sys/kernel/tracing` | limited | P4 | bounded-tree | Do not enable tracing; capture existing state under strict bounds. |
| TraceFS | `/sys/kernel/debug/tracing` | limited | P4 | bounded-tree | Legacy mount, same restrictions as tracefs. |
| BPF filesystem | `/sys/fs/bpf` | limited | P4 | bounded-tree | Pinned object metadata is useful; map/program dumps may be unsafe or large. |
| SecurityFS | `/sys/kernel/security` | conditional | P3 | bounded-tree | LSM, IMA, EVM, lockdown state when present and readable. |
| ConfigFS | `/sys/kernel/config` | conditional | P3 | bounded-tree | Dynamic kernel object configuration; bounded traversal. |
| Pstore | `/sys/fs/pstore` | collect | P2 | bounded-tree | Persistent crash evidence, high value. |
| Filesystem state | `/sys/fs` | conditional | P3 | bounded-tree | Filesystem-specific runtime state; avoid blind recursion. |
| Firmware | `/sys/firmware` | conditional | P3 | bounded-tree | Firmware runtime state where present. |
| Power | `/sys/power` | collect | P2 | raw-file-set | Small power state evidence. |
| Hardware monitor and thermal | `/sys/class/hwmon`, `/sys/class/thermal`, `/sys/class/power_supply` | collect | P2 | bounded-tree | Sensor, thermal, and power runtime state. |
| Graphics and accelerator | `/sys/class/drm`, `/sys/class/accel` | conditional | P3 | bounded-tree | Device and connector state when present. |
| RDMA and InfiniBand | `/sys/class/infiniband`, `/sys/class/infiniband_verbs` | conditional | P3 | bounded-tree | RDMA device state when present. |
| NVMe and storage class | `/sys/class/nvme`, `/sys/class/scsi_host`, `/sys/class/scsi_device`, `/sys/class/ata_link` | conditional | P3 | bounded-tree | Storage class state when present. |
| Watchdog, RTC, input, misc | `/sys/class/watchdog`, `/sys/class/rtc`, `/sys/class/input`, `/sys/class/dmi`, `/sys/class/leds`, `/sys/class/gpio`, `/sys/class/backlight` | conditional | P3 | bounded-tree | Miscellaneous device state when present. |
| Hypervisor and virtualization | `/sys/hypervisor`, `/sys/devices/virtual` | conditional | P3 | bounded-tree | VM and virtual device evidence. |
| IOMMU | `/sys/kernel/iommu_groups` | conditional | P3 | bounded-tree | IOMMU group evidence when present. |
| Bus, device, and driver | `/sys/bus`, `/sys/devices` | limited | P4 | bounded-tree | Very large tree; collect selected metadata and links under strict limits. |

## /run

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| systemd runtime | `/run/systemd` | conditional | P3 | bounded-tree | Runtime metadata and sockets; detailed state via native service collector. |
| User runtime | `/run/user` | limited | P3 | bounded-tree | Session runtime metadata; avoid copying user payloads. |
| Lock | `/run/lock` | collect | P2 | bounded-tree | Active lock evidence, low cost. |
| Daemon PID | `/run/*.pid` | collect | P2 | raw-file-set | Runtime PID evidence. |
| Container temporary state | `/run/docker`, `/run/containerd`, `/run/crio`, `/run/runc` | conditional | P3 | bounded-tree | Runtime socket and metadata discovery; detailed state via native protocol. |
| Udev runtime | `/run/udev` | conditional | P3 | bounded-tree | Device runtime database and queue state. |
| D-Bus runtime | `/run/dbus` | conditional | P3 | metadata | Socket and metadata discovery; protocol state via native implementation. |
| Resolver runtime | `/run/systemd/resolve` | conditional | P3 | bounded-tree | Resolver runtime metadata when present. |

## /dev

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Device node | `/dev` | limited | P2 | metadata | Capture node metadata, not arbitrary device contents. |
| Block device node | `/dev/block`, `/dev/disk` | collect | P2 | symlink-targets | Device mappings and stable identifiers. |
| Device mapper | `/dev/mapper` | collect | P2 | symlink-targets | Logical device mapping evidence. |
| Loop device | `/dev/loop*` | collect | P2 | metadata | Device node metadata only. |
| FUSE device | `/dev/fuse` | collect | P2 | metadata | Availability and metadata only. |
| TTY / PTY | `/dev/tty*`, `/dev/pts` | limited | P3 | metadata | Terminal inventory; do not read terminal streams. |
| Randomness and kernel message | `/dev/random`, `/dev/urandom` | collect | P2 | metadata | Metadata only; do not read random streams as evidence. |
| Randomness and kernel message | `/dev/kmsg` | conditional | P0 | bounded-window | Kernel messages through non-destructive bounded read policy. |
| Pseudo device | `/dev/null`, `/dev/zero`, `/dev/full`, `/dev/console`, `/dev/tty` | collect | P2 | metadata | Metadata only; do not read pseudo-device streams. |

## Kernel Ring Buffer

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Kernel error events | panic, oops, BUG, WARNING, GPF, stack trace windows | collect | P0 | bounded-window | High-value volatile kernel evidence. |
| OOM events | global OOM, cgroup OOM, victim, allocation failure, killed process windows | collect | P0 | bounded-window | Critical incident evidence. |
| Lockup and stall events | soft lockup, hard lockup, hung task, RCU stall, workqueue stall, blocked task windows | collect | P0 | bounded-window | Critical runtime-stall evidence. |
| Device and driver events | disk, NVMe, SCSI, USB, PCIe, network reset, firmware windows | collect | P2 | bounded-window | Device failure evidence. |
| Filesystem events | filesystem error, read-only remount, journal, NFS, FUSE, overlayfs windows | collect | P2 | bounded-window | Storage and filesystem incident evidence. |
| Network kernel events | link, TCP warning, neighbor failure, packet drop, queue timeout windows | collect | P2 | bounded-window | Network incident evidence. |

## System Event Log Store

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| System event window | boot, shutdown, daemon, resource warning, system error windows | exclude | P3 | skip | Log store protocol (journal/syslog) requires a native reader; outside kernel-focused scope. File-based bounded journal window capture is handled separately. |
| Kernel event mirror | recent kernel mirror window | exclude | P3 | skip | Journal protocol support out of scope; kernel ring buffer remains primary. |
| Service event window | service start, stop, restart, failure, exit, watchdog windows | exclude | P3 | skip | Service event log protocol out of scope. |
| Authentication and security | login, SSH, sudo, auth failure, permission denied windows | exclude | P3 | skip | Auth event log protocol out of scope. |
| Container event window | container start, stop, kill, OOM, restart windows | exclude | P3 | skip | Container event log protocol out of scope. |
| Audit event window | audit denial, syscall denial, privilege event windows | exclude | P3 | skip | Audit event log protocol out of scope. |

## Service Manager

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|--:|--:|--:|---|---|
| Manager state | manager, system, degraded, failed count, job queue | collect | P3 | native-protocol | Native D-Bus protocol support via custom D-Bus implementation. |
| Unit state | unit list, active/sub/load/result/dependency state | collect | P3 | native-protocol | Native D-Bus ListUnits call. |
| Service state | main process, exit status, restart, watchdog, cgroup, resource usage | collect | P3 | native-protocol | Native D-Bus unit property enumeration. |
| Socket unit state | socket unit, listening socket, accepted connection counters | collect | P3 | native-protocol | Native D-Bus unit property enumeration. |
| Timer unit state | timer state, last/next trigger, associated unit | collect | P3 | native-protocol | Native D-Bus unit property enumeration. |
| Failed / degraded state | failed units, degraded state, failed jobs | collect | P3 | native-protocol | Native D-Bus ListUnitsFiltered call. |

## Netlink

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Link | link, flags, MTU, state, qdisc relation | collect | P2 | native-netlink | Native replacement for `ip link`. |
| Address | IPv4/IPv6 addresses, scope, lifetimes | collect | P2 | native-netlink | Native replacement for `ip addr`. |
| Route | route tables, default route, rules, metrics, multipath | collect | P2 | native-netlink | Native replacement for `ip route` and `ip rule`. |
| Neighbor | ARP/IPv6 neighbors, state, failed neighbors | collect | P2 | native-netlink | Native neighbor evidence. |
| Socket diagnostic | TCP, UDP, Unix sockets, owner, queues, memory | conditional | P3 | native-netlink | Valuable but can be large; budgeted socket dump. |
| Conntrack | entries, counters, NAT relation, timeouts | conditional | P3 | native-netlink | Requires kernel support and can be large. |
| Traffic control | qdisc, class, filter, backlog, drop counters | conditional | P3 | native-netlink | Useful under network incidents, requires native support. |
| XFRM | state, policy, IPsec security associations | conditional | P3 | native-netlink | Capture when IPsec/XFRM state exists. |

## Security / Audit Subsystem

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Process security | capabilities, seccomp, NoNewPrivs, LSM process context | collect | P1 | raw-file-set | Derived from procfs process state. |
| SELinux | enforcing, policy loaded, process contexts, AVC denial window | conditional | P3 | bounded-tree | Capture when SELinux/securityfs/log support is present. |
| AppArmor | profile state, process profile, denial window | conditional | P3 | bounded-tree | Capture when AppArmor/securityfs/log support is present. |
| Audit | audit daemon, backlog, lost counter, recent audit window | deferred-native | P3 | native-protocol | Native audit support required; no external tools. |
| Privilege | sudo session, auth failure, privilege escalation windows | deferred-native | P3 | native-protocol | Native log/session support required. |

## User / Session Database

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Login session | logged-in user, login time, TTY, remote host | conditional | P3 | native-protocol | Capture through native session database support. |
| SSH session | SSH session, remote address, TTY relation | conditional | P3 | raw-file-set | Capture from process/session evidence and native records where available. |
| TTY / PTY | active TTY, active PTY, owner relation | conditional | P3 | metadata | Capture metadata only; do not read streams. |
| logind | sessions, user state, seat state, linger state | collect | P3 | native-protocol | Native D-Bus logind protocol via custom D-Bus implementation. |
| Interactive process relation | foreground process group, shell relation, interactive process | conditional | P3 | raw-file-set | Derived from procfs/session metadata when available. |

## Scheduler / Job Runtime

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Cron runtime | running cron job and recent cron event windows | deferred-native | P3 | native-protocol | Runtime evidence only; no static crontab capture. |
| systemd timer runtime | timer state, last/next trigger, associated unit | deferred-native | P3 | native-protocol | Covered by native service manager collector. |
| at / batch runtime | queued/running jobs, state, owner relation | conditional | P3 | bounded-tree | Runtime queue state only; avoid static schedule expansion. |
| External job runtime | external job state, owner, result | deferred-native | P3 | native-protocol | Requires source-specific native integration. |

## Time Synchronization Subsystem

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| System clock | wall clock, monotonic clock, boot time, time jump window | collect | P0 | metadata | Essential capture context via /proc/uptime, /proc/stat (btime), /etc/localtime. |
| NTP | synchronized state, source, offset, jitter, stratum, leap | exclude | P3 | skip | NTP protocol support out of scope for kernel-focused collection. |
| chrony | tracking, sources, activity | exclude | P3 | skip | chrony protocol support out of scope. |
| systemd-timesyncd | timesyncd state | exclude | P3 | skip | systemd-timesyncd D-Bus protocol out of scope; file-based state from /run/systemd/timesync is handled separately. |
| PTP | clock state, port state, master offset, grandmaster relation | conditional | P3 | bounded-tree | Capture kernel-exposed PTP state where present; protocol support later. |

## Crash Dump Store

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| User-space core dump | records, crashed process metadata, signal, timestamp, core file metadata | limited | P3 | metadata-only | Do not copy large core payloads by default. |
| systemd-coredump | coredump record and journal metadata | deferred-native | P3 | native-protocol | Native coredump/journal support required. |
| Kernel crash dump | kdump state, crash kernel, vmcore metadata, panic records | limited | P3 | metadata-only | Do not copy large vmcore payloads by default. |
| Recent crash record | recent user-space and kernel crash windows | conditional | P3 | bounded-window | Capture bounded records where native source exists. |

## Hardware Management Interface

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---:|---:|---|---|
| Disk health | health, temperature, media error, lifetime counters | deferred-native | P4 | native-protocol | No `smartctl`; native safe implementation required. |
| NVMe management | health, SMART log, error log | deferred-native | P4 | native-protocol | No `nvme`; native safe implementation required. |
| RAID controller | virtual drive, physical drive, rebuild, battery/cache state | deferred-native | P4 | native-protocol | Vendor interfaces require native safe design. |
| IPMI / BMC / Redfish | chassis power, sensors, fans, PSU, SEL | deferred-native | P4 | native-protocol | No `ipmitool`; native safe implementation required. |
| GPU management | device, utilization, memory, temperature, process relation | deferred-native | P4 | native-protocol | No vendor CLI; collect sysfs evidence separately. |
| RDMA / InfiniBand | device, port, counters, congestion counters | conditional | P3 | bounded-tree | Collect Linux-exposed sysfs state; advanced management deferred. |
| Sensors / thermal / power | temperature, fan, voltage, thermal zone, power state | collect | P2 | bounded-tree | Covered through `/sys/class/hwmon`, thermal, and power_supply. |

## Container Runtime

| Domain | Coverage unit | Decision | Priority | Mode | Rationale |
|---|---|---|---|---|---|
| Runtime daemon | daemon state, socket state, version, error state | exclude | P3 | skip | Container daemon API protocol out of scope for kernel-focused collection. |
| Container object | list, state, lifecycle, exit, restart metadata | exclude | P3 | skip | Container object enumeration protocol out of scope. |
| Container process | init process, host PID relation, task state | conditional | P3 | raw-file-set | Host-side procfs evidence can be collected before runtime protocol support. |
| Container resource | CPU, memory, PID, block I/O, network I/O state | conditional | P3 | bounded-tree | Host cgroup evidence first; runtime API later. |
| Container namespace | PID, network, mount, IPC, UTS namespace relation | conditional | P3 | symlink-targets | Host-side namespace evidence via procfs. |
| Container cgroup | container cgroup relation | conditional | P3 | bounded-tree | Host cgroup evidence via `/sys/fs/cgroup`. |
| Container event / log | recent events, stdout window, stderr window | exclude | P3 | skip | Container log protocol out of scope. |

## Explicit Exclusions

The following are outside the core capture target even when present on a host:

| Exclusion | Decision | Priority | Mode | Rationale |
|---|---:|---:|---|---|
| Static system configuration files as a primary target | exclude | NA | skip | Outside runtime raw snapshot scope. |
| Application configuration and business data | exclude | NA | skip | Application/business scope, not Linux runtime evidence. |
| Source code files and package manager databases | exclude | NA | skip | Static host content, not incident runtime state. |
| Container image contents | exclude | NA | skip | Image artifact scope, not local runtime state. |
| Database internal runtime state | exclude | NA | skip | Database-specific tooling scope. |
| Language runtime internals such as JVM, Go, Python, Node.js, .NET, BEAM | exclude | NA | skip | Language-specific tooling scope. |
| Application debug endpoints and application metrics systems | exclude | NA | skip | Application observability scope. |
| Kubernetes API and orchestration control-plane state | exclude | NA | skip | Orchestration control-plane scope. |
| Cloud provider control-plane APIs | exclude | NA | skip | Cloud control-plane scope. |
| Full long-term historical logs | exclude | NA | skip | Unbounded and outside incident snapshot scope; bounded windows only. |
| External command output | exclude | NA | skip | Violates native Rust collection policy. |
| Large core, vmcore, or kernel memory payloads by default | exclude | NA | skip | Too large and risky for incident-time capture; metadata only. |

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
