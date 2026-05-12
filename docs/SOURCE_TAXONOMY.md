# Source Taxonomy

## Purpose

This document adapts the Linux runtime raw snapshot taxonomy into the project
design. It describes collection source families, boundaries, and collection
policy. The taxonomy is not a promise that every object is implemented in the
initial release; it is the map used by the planner and collectors.

The model has three layers:

```text
L1. Root Collection Source
  L2. Internal Source Domain
    L3. Raw Snapshot Object
```

Runtime execution adds:

```text
L4. Capture Task
```

## L3 Object Policy

This taxonomy owns source boundaries and object families. It intentionally avoids
field-level duplication for raw files whose internal fields are already preserved
as bytes. For example, `/proc/stat` is one raw snapshot object family even though
it contains many counters.

An L3 raw snapshot object is considered assigned when exactly one row in the
Coverage Decision Matrix names the object, its path family, or its source domain
as the primary coverage unit. If two rows can cover the same object, one row must
be documented as a fallback, supplement, or derived view.

This keeps the design auditable without turning the taxonomy into a copy of Linux
kernel documentation.

## Scope Boundary

Included sources are Linux system-level runtime data that can change or
disappear after reboot, process exit, connection close, cgroup removal, namespace
change, device change, kernel object reclamation, or service/container runtime
change.

Excluded sources are static configuration, application data, language runtimes,
database internals, Kubernetes control-plane state, cloud APIs, package manager
databases, source code, and full historical logs.

## Collection Strategy: Auto-Discovery with Blacklist

The project favors automatic discovery over hardcoded path lists. The model:

1. **Scan** the root directory or netlink protocol family to discover available objects.
2. **Blacklist** known-dangerous or write-interface paths (explicit exclusion).
3. **Deprioritize** high-cost objects (collect at lower priority with tighter bounds).
4. **Collect** everything else within bounded limits (timeout, bytes, depth, files).

This avoids three problems:
- **Missing evidence** when a new kernel version introduces a new runtime file.
- **Noise** (`not_found` entries) from hardcoded paths that don't exist on a given host.
- **Maintenance burden** of updating path lists per kernel release.

Blacklists are narrow: they only exclude paths that are write-only, destructive to
read, or expose kernel/user memory in bulk.

## L1 Source Families

```text
1. /proc
2. /sys
3. /run
4. /dev
5. kernel ring buffer
6. system event log store
7. service manager
8. netlink
9. security / audit subsystem
10. user / session database
11. scheduler / job runtime
12. time synchronization subsystem
13. crash dump store
14. hardware management interface
```

Subpaths such as `/proc/net`, `/proc/<pid>`, `/sys/fs/cgroup`,
`/sys/class/net`, `/run/systemd`, and `/sys/fs/pstore` remain L2 domains under
their root source. They are not promoted to L1.

---

## 1. /proc

Purpose: Primary kernel and process runtime state.

Collection policy: **Auto-discover** — scan `/proc` root for non-PID files,
collect all discovered files. For per-process directories, scan each PID
directory, collect all files except blacklisted entries.

**Global blacklist** (never read these):
- `kcore` — kernel memory image (large, sensitive)
- `kallsyms` — kernel symbol addresses (security-sensitive)
- `kpagecount`, `kpageflags`, `kpagecgroup` — physical page metadata (large)
- `sysrq-trigger` — **write interface** (triggers kernel actions)

**Per-process blacklist** (never read these):
- `mem` — process memory image (large, security-sensitive)
- `pagemap` — process page table (very large)
- `clear_refs` — **write interface** (mutates kernel state)
- `oom_adj` — **write interface** (deprecated, mutates OOM score)
- `coredump_filter` — **write interface**
- `attr/*` — security attribute **write interfaces**
- `uid_map`, `gid_map`, `projid_map` — **write interfaces** (namespace config)
- `setgroups` — **write interface**
- `reclaim` — **write interface**

**Deprioritized** (collect with tighter limits / lower priority):
- `smaps` — per-process memory map details (very large)
- `smaps_rollup` — aggregate smaps (collect at P2)
- `task/<tid>/children` — nested process hierarchy

### L2 Domains

#### 1.1 System overview (system_overview)
Global host state files. Discovered by scanning `/proc` root.

L3 objects: `loadavg`, `uptime`, `version`, `cmdline`, `cpuinfo`, `swaps`,
`consoles`, `execdomains`, `profile`

#### 1.2 CPU and scheduler (cpu_scheduler)
Scheduler runtime counters.

L3 objects: `stat`, `schedstat`, `sched_debug` (P4 conditional)

#### 1.3 Memory and VM (memory_vm)
Virtual memory subsystem state.

L3 objects: `meminfo`, `vmstat`, `slabinfo`, `zoneinfo`, `buddyinfo`,
`pagetypeinfo` (P3 conditional)

#### 1.4 PSI pressure (psi_pressure)
Pressure Stall Information — direct runtime pressure signals.

L3 objects: `pressure/cpu`, `pressure/memory`, `pressure/io`,
`pressure/irq` (kernel ≥ 6.1)

#### 1.5 Interrupt and softirq (interrupt_softirq)
Interrupt distribution and softirq activity.

L3 objects: `interrupts`, `softirqs`, `irq/*` (bounded-tree)

#### 1.6 Block device statistics (block)
Block layer I/O evidence.

L3 objects: `diskstats`, `partitions`, `mdstat`

#### 1.7 Network protocol stack (network)
Network protocol visibility through `/proc/net/`.

L3 objects (all discovered by scanning `/proc/net/`):
`dev`, `snmp`, `netstat`, `tcp`, `tcp6`, `udp`, `udp6`, `unix`, `raw`, `raw6`,
`packet`, `arp`, `route`, `ipv6_route`, `dev_mcast`, `igmp`, `igmp6`,
`tls`, `tls_stat`, `mptcp`, `mptcp_net/*`, `sctp/*`, `dccp`,
`smcr`, `rpc/*`, `rds/*`, `nfsfs/*`, `l2tp/*`, `pppoe`,
`sockstat`, `sockstat6`, `protocols`, `softnet_stat`, `ptype`,
`wireless`, `if_inet6`, `netfilter/*`

#### 1.8 IPC (ipc)
System V inter-process communication objects.

L3 objects: `sysvipc/shm`, `sysvipc/msg`, `sysvipc/sem`

#### 1.9 File locks (file_locks)
Active file lock evidence.

L3 objects: `locks`

#### 1.10 Mount and filesystem view (mount_fs)
Mount table and filesystem registration.

L3 objects: `mounts`, `self/mountinfo`, `self/mountstats`, `filesystems`,
`fs/*`, `fs/nfsfs/*`, `fs/nfsd/*`, `fs/cifs/*`, `fs/xfs/*`, `fs/btrfs/*`

#### 1.11 Kernel modules (modules)
Loaded kernel module inventory.

L3 objects: `modules`

#### 1.12 Kernel resource and hardware maps (hardware_map)
Physical resource and device maps.

L3 objects: `iomem`, `ioports`, `devices`, `misc`, `cgroups`,
`dma`, `mtrr`, `device-tree/*`

#### 1.13 Crypto, keys, and timer (crypto_keys)
Kernel cryptography and key retention state.

L3 objects: `crypto`, `key-users`

#### 1.14 Process and thread (process)
Per-process runtime evidence.

L3 objects: All files under `/proc/<pid>/` **except blacklisted entries**.
Special handling for: `fd/*` (symlink-to-target listing), `fdinfo/*`,
`ns/*` (namespace symlinks), `task/*/` (thread directories),
`net/*` (per-process network namespace),
`mounts`, `mountinfo` (per-process mount view)

#### 1.15 /proc/sys sysctl (sysctl)
Kernel runtime parameters. All files are safe read-only text.

L3 objects: Full recursive tree under `/proc/sys/` with depth limit.
Includes: `kernel/*`, `vm/*`, `fs/*`, `net/*`, `user/*`, `debug/*`,
`dev/*`, `abi/*`

---

## 2. /sys

Purpose: Kernel object model, devices, drivers, classes, cgroups, firmware state,
and subsystem runtime state.

Collection policy: **Auto-discover** — bounded-depth recursive scan. For each
device directory, discover all attribute files. Do not read device content (inode
metadata only for device nodes).

**Global blacklist** (write interfaces that mutate kernel state):
- `uevent` — reading may consume a uevent
- `bind`, `unbind` — driver binding control
- `probe` — trigger device probe
- `reset`, `trigger` — device reset
- `store`, `config` — write-only configuration
- `state`, `disk` (under `/sys/power/`) — may trigger suspend/hibernate
- `tun_flags` — virtio network write interface

### L2 Domains

#### 2.1 Network devices (network_device)
Interface attributes, statistics, queues, and device links.

Path: `/sys/class/net/*/`, `/sys/devices/virtual/net/*/`
All attribute files auto-discovered per interface.

#### 2.2 Block devices (block_device)
Block attributes, queues, stats, partitions, mapper relations.

Path: `/sys/block/*/`, `/sys/block/*/queue/`,
`/sys/block/zram*/` (compressed RAM block devices)

#### 2.3 CPU devices (cpu_device)
CPU topology, frequency, idle states, cache, vulnerabilities.

Path: `/sys/devices/system/cpu/*/`,
`/sys/devices/system/cpu/cpu*/topology/`,
`/sys/devices/system/cpu/cpu*/cache/`,
`/sys/devices/system/cpu/cpufreq/`,
`/sys/devices/system/cpu/cpuidle/`,
`/sys/devices/system/cpu/vulnerabilities/` (CPU security — **critical evidence**),
`/sys/devices/system/cpu/smt/`,
`/sys/devices/system/cpu/microcode/`

#### 2.4 NUMA (numa)
NUMA node memory, distance, hugepage, compaction evidence.

Path: `/sys/devices/system/node/*/`

#### 2.5 Cgroup (cgroup)
Cgroup hierarchy and controller state.

Path: `/sys/fs/cgroup/` — bounded tree, auto-discover controllers and stats.
Key files auto-discovered: `cgroup.procs`, `memory.current`, `memory.stat`,
`memory.pressure`, `cpu.stat`, `cpu.pressure`, `io.stat`, `io.pressure`,
`pids.current`, `cgroup.controllers`, `cgroup.events`, `cgroup.freeze`

#### 2.6 Kernel modules (module)
Module parameters, holders, and sections.

Path: `/sys/module/*/`, `/sys/module/*/parameters/`,
`/sys/module/*/holders/`, `/sys/module/*/sections/`,
`/sys/module/*/notes/`

#### 2.7 Kernel state (kernel)
General kernel runtime state and memory management internals.

Path: `/sys/kernel/`, `/sys/kernel/mm/`,
`/sys/kernel/mm/transparent_hugepage/`,
`/sys/kernel/mm/ksm/`, `/sys/kernel/mm/hugepages/`,
`/sys/kernel/mm/swap/`, `/sys/kernel/mm/compaction/`,
`/sys/kernel/mm/cleancache/`,
`/sys/kernel/livepatch/`, `/sys/kernel/slab/`

#### 2.8 Filesystem-specific state (filesystem)
Per-filesystem-type runtime statistics.

Path: `/sys/fs/`, `/sys/fs/btrfs/`, `/sys/fs/xfs/`,
`/sys/fs/ext4/`, `/sys/fs/fuse/connections/`,
`/sys/fs/selinux/` (security domain)

#### 2.9 Firmware (firmware)
Firmware and hardware description tables.

Path: `/sys/firmware/`, `/sys/firmware/dmi/`, `/sys/firmware/acpi/`,
`/sys/firmware/efi/` (efivars conditional P4),
`/sys/firmware/devicetree/`, `/sys/firmware/memmap/`

#### 2.10 Power (power)
System power state evidence. Read-only files only.

Path: `/sys/power/` (excluding `state`, `disk`)

#### 2.11 Hardware monitor and thermal (hwmon_thermal)
Sensor, thermal zone, cooling device, and power supply state.

Path: `/sys/class/hwmon/*/`, `/sys/class/thermal/*/`,
`/sys/class/thermal/cooling_device*/`,
`/sys/class/power_supply/*/`,
`/sys/devices/virtual/thermal/*/`

#### 2.12 Graphics and accelerator (graphics)
GPU DRM device and compute accelerator state.

Path: `/sys/class/drm/*/`, `/sys/class/accel/*/`

#### 2.13 RDMA and InfiniBand (rdma)
RDMA and InfiniBand device state.

Path: `/sys/class/infiniband/*/`, `/sys/class/infiniband_verbs/*/`

#### 2.14 NVMe and storage classes (storage)
NVMe, SCSI, ATA, Fibre Channel, and SAS device state.

Path: `/sys/class/nvme/*/`, `/sys/class/nvme-subsystem/*/`,
`/sys/class/scsi_host/*/`, `/sys/class/scsi_device/*/`,
`/sys/class/ata_link/*/`, `/sys/class/ata_device/*/`,
`/sys/class/fc_host/*/`, `/sys/class/sas_device/*/`,
`/sys/class/enclosure/*/`

#### 2.15 Miscellaneous devices (misc_devices)
Watchdog, RTC, input, GPIO, I2C, SPI, TTY, and other device class state.

Path: `/sys/class/watchdog/*/`, `/sys/class/rtc/*/`,
`/sys/class/input/*/`, `/sys/class/dmi/*/`,
`/sys/class/leds/*/`, `/sys/class/backlight/*/`,
`/sys/class/gpio/*/`, `/sys/class/tty/*/`,
`/sys/class/vtconsole/*/`, `/sys/class/i2c-adapter/*/`,
`/sys/class/i2c-dev/*/`, `/sys/class/spi_master/*/`,
`/sys/class/regulator/*/`, `/sys/class/extcon/*/`,
`/sys/class/devcoredump/*/`, `/sys/class/devlink/*/`,
`/sys/class/pci_bus/*/`, `/sys/class/mei/*/`,
`/sys/class/mic/*/`, `/sys/class/uwb_rc/*/`,
`/sys/class/wmi_bus/*/`

#### 2.16 Debug and tracing (debug_trace)
DebugFS, TraceFS — policy-sensitive, limited P4.

Path: `/sys/kernel/debug/` (selected safe subtrees),
`/sys/kernel/tracing/` (read existing state, do not enable tracing),
`/sys/kernel/debug/tracing/` (legacy mount)

#### 2.17 BPF filesystem (bpffs)
Pinned BPF object metadata.

Path: `/sys/fs/bpf/` (metadata only, limited P4)

#### 2.18 Security filesystem (security_fs)
LSM, IMA, EVM, and lockdown state.

Path: `/sys/kernel/security/`, `/sys/kernel/security/lsm`,
`/sys/kernel/security/lockdown`, `/sys/kernel/security/ima/`,
`/sys/kernel/security/evm/`, `/sys/kernel/security/tomoyo/`

#### 2.19 ConfigFS (configfs)
Dynamic kernel object configuration — metadata-oriented, conditional P3.

Path: `/sys/kernel/config/`

#### 2.20 Pstore (pstore)
Persistent crash and panic evidence — high value, collect early.

Path: `/sys/fs/pstore/`

#### 2.21 Virtualization and hypervisor (virtualization)
Hypervisor type, virtual devices, KVM module state.

Path: `/sys/hypervisor/`, `/sys/devices/virtual/`,
`/sys/class/misc/kvm/`, `/sys/module/kvm*/parameters/`,
`/sys/module/kvm_intel/parameters/`, `/sys/module/kvm_amd/parameters/`

#### 2.22 IOMMU (iommu)
IOMMU group topology for device passthrough analysis.

Path: `/sys/kernel/iommu_groups/`, `/sys/class/iommu/*/`

#### 2.23 Bus, device, and driver trees (bus_device_driver)
Large hardware topology trees — limited P4 with strict bounds.

Path: `/sys/bus/*/devices/`, `/sys/devices/`

#### 2.24 Memory error detection (edac)
EDAC memory controller error counters and MCE state.

Path: `/sys/devices/system/edac/`,
`/sys/devices/system/machinecheck/`

---

## 3. /run

Purpose: Boot-lifetime runtime state created by services and daemons.

Collection policy: **Auto-discover** — scan `/run` root for discrete files,
collect all. Subdirectories traversed with bounded depth (max 3) and file count
limits. Socket files recorded as metadata only (existence + permissions).

### L2 Domains

#### 3.1 systemd runtime (systemd_runtime)
Service manager runtime state, seats, sessions, users, machines, network
interfaces, password agents, and generator outputs.

Path: `/run/systemd/`, `/run/systemd/seats/`,
`/run/systemd/sessions/`, `/run/systemd/users/`,
`/run/systemd/machines/`, `/run/systemd/netif/`,
`/run/systemd/ask-password/`, `/run/systemd/generator*/`

#### 3.2 User runtime (user_runtime)
Per-user D-Bus, systemd user instances.

Path: `/run/user/*/` (first-level files and known subdirs only)

#### 3.3 Locks (lock)
Active file lock state.

Path: `/run/lock/`

#### 3.4 Daemon PID files (daemon_pid)
Runtime process ID evidence from running daemons.

Path: `/run/*.pid` (scanned by glob)

#### 3.6 udev runtime (udev_runtime)
Device runtime database and queue state.

Path: `/run/udev/`

#### 3.7 D-Bus runtime (dbus_runtime)
System D-Bus socket presence and metadata.

Path: `/run/dbus/`

#### 3.8 Resolver runtime (resolver_runtime)
Systemd-resolved runtime state.

Path: `/run/systemd/resolve/`

#### 3.9 Other /run paths (misc_run)
Additional runtime state files.

Path: `/run/utmp` (login records), `/run/faillock/` (login failures),
`/run/mount/utab` (user mounts),
`/run/NetworkManager/`, `/run/chrony/`

---

## 4. /dev

Purpose: Runtime device namespace and device node metadata.

Collection policy: **Metadata-only** — capture device node listings, symlink
targets, major/minor numbers, permissions, and file types. Do not read device
contents. This is the correct and safe approach; auto-discovery of device
*contents* would be dangerous.

**Absolute blacklist** (never open these, even for metadata beyond stat):
- `/dev/cpu/*/msr` — Model-Specific Register access (security risk)
- `/dev/mem` — physical memory access
- `/dev/kmem` — kernel virtual memory access
- `/dev/port` — I/O port access
- `/dev/watchdog*` — opening starts the watchdog timer
- `/dev/sg*`, `/dev/bsg/*` — SCSI generic (can send arbitrary commands)

### L2 Domains

#### 4.1 Device nodes (device_node)
Top-level `/dev/` directory listing with metadata for each entry.

#### 4.2 Block device nodes (block_dev)
Persistent block device identifiers and mappings.

Path: `/dev/block/`, `/dev/disk/`, `/dev/mapper/`

#### 4.3 Loop devices (loop_dev)
Loopback block device metadata.

Path: `/dev/loop*`

#### 4.4 FUSE device (fuse_dev)
FUSE availability metadata.

Path: `/dev/fuse`

#### 4.5 TTY and PTY (tty_pty)
Terminal device inventory (metadata only — no stream reads).

Path: `/dev/tty*`, `/dev/pts/`

#### 4.6 Randomness and kernel message (random_kmsg)
RNG device metadata. `/dev/kmsg` routed to kernel ring buffer collector.

Path: `/dev/random`, `/dev/urandom`, `/dev/kmsg`

#### 4.7 Pseudo devices (pseudo_dev)
Standard pseudo-device metadata.

Path: `/dev/null`, `/dev/zero`, `/dev/full`, `/dev/console`, `/dev/tty`

#### 4.8 Virtualization and passthrough devices (virt_dev)
KVM, VFIO, vhost, uinput, uhid device metadata.

Path: `/dev/kvm`, `/dev/vfio/*`, `/dev/vhost*`,
`/dev/uinput`, `/dev/uhid`

---

## 5. Kernel Ring Buffer

Purpose: Volatile kernel event evidence that may be overwritten or lost after reboot.

Collection policy: Non-blocking bounded window read from `/dev/kmsg`.
Respect `dmesg_restrict` kernel setting.

### L2 Domains

#### 5.1 Kernel error events (kernel_errors)
panic, oops, BUG, WARNING, GPF, stack trace windows.

#### 5.2 OOM events (oom_events)
Global OOM, cgroup OOM, victim, allocation failure, killed process windows.

#### 5.3 Lockup and stall events (lockup_stall)
soft lockup, hard lockup, hung task, RCU stall, workqueue stall.

#### 5.4 Device and driver events (device_events)
disk, NVMe, SCSI, USB, PCIe, network reset, firmware windows.

#### 5.5 Filesystem events (fs_events)
filesystem error, read-only remount, journal, NFS, FUSE, overlayfs.

#### 5.6 Network kernel events (net_events)
link state, TCP warning, neighbor failure, packet drop, queue timeout.

---

## 6. System Event Log Store

Purpose: Bounded runtime event windows from local logging facilities.

Collection policy: **Auto-discover available log paths** — check existence,
read bounded windows (file-size limited, not time-range limited).
No external command invocation (`journalctl` etc.).

### L2 Domains

#### 6.1 journald runtime (journal_run)
Bounded window from current-boot journal files.

Path: `/run/log/journal/*/`

#### 6.2 journald persistent (journal_var)
Bounded window from persistent journal files.

Path: `/var/log/journal/*/`

#### 6.3 Traditional syslog (syslog)
Bounded windows from standard syslog daemon output files.

Path: `/var/log/syslog`, `/var/log/messages`, `/var/log/kern.log`,
`/var/log/auth.log`, `/var/log/dmesg`

#### 6.4 Last login records (lastlog)
Last login timestamps per user.

Path: `/var/log/lastlog`

---

## 7. Service Manager

Purpose: Runtime state maintained by the local init or service management system
(systemd primary target).

Collection policy: Native D-Bus protocol. No external commands.

### L2 Domains

#### 7.1 Manager state (manager)
systemd version, features, architecture, virtualization, control group info.

D-Bus: `org.freedesktop.systemd1.Manager` properties.

#### 7.2 Unit state (units)
Active, sub, load, result, and dependency state for all units.

D-Bus: `ListUnits`, `GetUnit`, `GetUnitFileState`.

#### 7.3 Service state (service)
Main process, exit status, restart, watchdog, cgroup, resource usage.

D-Bus: Unit property enumeration on service units.

#### 7.4 Socket unit state (socket_units)
Listening sockets, accepted connection counters.

#### 7.5 Timer unit state (timer_units)
Last/next trigger, associated unit.

#### 7.6 Failed and degraded state (failed_degraded)
Failed units, degraded manager state, failed jobs.

D-Bus: `ListUnitsFiltered`, `ListJobs`.

---

## 8. Netlink

Purpose: Native structured runtime state for networking and selected kernel subsystems.

Collection policy: **Auto-discover supported netlink families** — attempt
dump for each known family, mark unsupported families explicitly. Preserve raw
binary responses. No external commands (`ip`, `ss`, `tc`, etc.).

### L2 Domains

#### 8.1 Link (link)
Interfaces, flags, MTU, state, qdisc relations. `RTM_GETLINK`.

#### 8.2 Address (address)
IPv4/IPv6 addresses, scopes, lifetimes. `RTM_GETADDR`.

#### 8.3 Route (route)
Route tables, rules, metrics, multipath. `RTM_GETROUTE`.

#### 8.4 Neighbor (neighbor)
ARP/IPv6 neighbor cache, state, failures. `RTM_GETNEIGH`.

#### 8.5 Socket diagnostic (sock_diag)
TCP, UDP, Unix socket state, owners, queues, memory. `NETLINK_SOCK_DIAG`.

#### 8.6 Conntrack (conntrack)
Connection tracking entries, counters, NAT relations. `NETLINK_NETFILTER`.

#### 8.7 Traffic control (traffic_control)
Qdisc, class, filter, backlog, drop counters.

#### 8.8 XFRM (xfrm)
IPsec security association and policy state.

#### 8.9 Extended protocol families (extended_netlink)
Auto-probed, conditional P3:
`NETLINK_NETFILTER` (nftables), `NETLINK_IP6_FW`, `NETLINK_CRYPTO`,
`NETLINK_RDMA`, `NETLINK_NVME` (NVMe-oF), `NETLINK_FOU`,
`NETLINK_TIPC`, `NETLINK_ECRYPTFS`, `NETLINK_SMC`

---

## 9. Security / Audit Subsystem

Purpose: Runtime state from Linux security modules, audit, and privilege subsystems.

Collection policy: Filesystem-based for procfs/securityfs visible state.
No external tools.

### L2 Domains

#### 9.1 Process security (process_security)
Capabilities, seccomp filter state, NoNewPrivs, LSM contexts.

Derived from `/proc/<pid>/status`, `/proc/<pid>/attr/current` (read-only).

#### 9.2 SELinux (selinux)
Enforcing mode, policy version, loaded policy metadata.

Path: `/sys/fs/selinux/`.

#### 9.3 AppArmor (apparmor)
Profile state, process profiles.

Path: `/sys/kernel/security/apparmor/`,
`/sys/module/apparmor/parameters/`.

#### 9.4 Audit (audit)
Login UID, session ID.

Path: `/proc/self/loginuid`, `/proc/self/sessionid`.

#### 9.5 IMA and EVM (ima_evm)
Integrity Measurement Architecture policy and PCR values.
Extended Verification Module state.

Path: `/sys/kernel/security/ima/`, `/sys/kernel/security/evm/`.

#### 9.6 Lockdown (lockdown)
Kernel lockdown level (none, integrity, confidentiality).

Path: `/sys/kernel/security/lockdown`.

#### 9.7 Landlock (landlock)
Landlock LSM sandboxing detection via `/proc/<pid>/status`.

---

## 10. User / Session Database

Purpose: Logged-in users, sessions, terminals, seats, and interactive process
relationships.

Collection policy: D-Bus logind protocol + runtime file evidence.
No external commands (`who`, `w`, `loginctl`, etc.).

### L2 Domains

#### 10.1 logind sessions (logind)
Active sessions, users, and seats.

D-Bus: `org.freedesktop.login1.Manager` — `ListSessions`, `ListUsers`,
`GetSession`, `GetUser`.

#### 10.2 Login records (login_records)
Traditional login evidence.

Path: `/run/utmp`.

#### 10.3 SSH sessions (ssh_sessions)
SSH session detection via process command line and environment.

#### 10.4 User runtime directories (user_runtime)
Active user detection via `/run/user/*/` presence.

---

## 11. Scheduler / Job Runtime

Purpose: Runtime state of local job schedulers, not static schedule configuration.

Collection policy: Bounded filesystem collection from known spool directories.
No external commands.

### L2 Domains

#### 11.1 systemd timers (systemd_timers)
Timer unit state via D-Bus `ListUnits` filtered by timer type.

#### 11.2 Cron (cron)
Per-user crontab files and cron job state.

Path: `/var/spool/cron/crontabs/`, `/var/spool/cron/atjobs/`.

#### 11.3 Anacron (anacron)
Anacron job timestamps.

Path: `/var/spool/anacron/`.

#### 11.4 At jobs (at_jobs)
Queued at(1) jobs.

Path: `/var/spool/at/`.

#### 11.5 fcron (fcron)
fcron scheduler state.

Path: `/var/spool/fcron/`.

---

## 12. Time Synchronization Subsystem

Purpose: Runtime state of system clocks and synchronization services.

Collection policy: Filesystem evidence + kernel clock interfaces.
No external commands (`chronyc`, `timedatectl`).

### L2 Domains

#### 12.1 System clock (system_clock)
Wall clock, monotonic clock, boot time.

Source: `/proc/uptime`, `/proc/stat` (btime field),
`/etc/localtime`, `/etc/timezone`, `/etc/adjtime`.

#### 12.2 systemd-timesyncd (timesyncd)
timesyncd runtime state.

Path: `/run/systemd/timesync/`.

#### 12.3 Clock source (clock_source)
Current and available kernel clock sources.

Path: `/sys/devices/system/clocksource/clocksource0/`.

#### 12.4 PTP (ptp)
Precision Time Protocol hardware clock devices.

Path: `/sys/class/ptp/*/`.

---

## 13. Crash Dump Store

Purpose: Runtime crash metadata and persisted crash artifacts.

Collection policy: Metadata-first. Do not copy large core/vmcore payloads by
default. Preserve enough references for manual retrieval.

### L2 Domains

#### 13.1 kdump (kdump)
Kexec crash kernel state.

Path: `/sys/kernel/kexec_crash_loaded`, `/sys/kernel/kexec_crash_size`.

#### 13.2 Core dump (coredump)
Core dump pattern and recent core files.

Path: `/proc/sys/kernel/core_pattern`, `/var/lib/systemd/coredump/`.

#### 13.3 Pstore (pstore)
Persistent crash and panic records.

Path: `/sys/fs/pstore/`.

#### 13.4 Crash log directories (crash_log)
Distribution crash report directories — metadata only.

Path: `/var/crash/`.

---

## 14. Hardware Management Interface

Purpose: Runtime hardware state exposed outside ordinary `/sys` paths.

Collection policy: Prefer Linux-exposed runtime state under `/sys` and native
kernel interfaces. Mark unavailable hardware management features as
`deferred-native` until native Rust access is designed.

### L2 Domains

#### 14.1 Sensors, thermal, power (sensors)
Temperature, fan, voltage, thermal zone, cooling device, power state.

Path: `/sys/class/hwmon/*/`, `/sys/class/thermal/*/`,
`/sys/class/power_supply/*/`.

#### 14.2 Disk health (disk_health)
SCSI/SAS device attributes, IDE device state.

Path: `/sys/block/*/device/`, `/proc/ide/*/`.

#### 14.3 Software RAID (md_raid)
MD RAID array state.

Path: `/proc/mdstat`.

#### 14.4 NVMe management (nvme_mgmt)
deferred-native — requires safe NVMe admin command interface.

#### 14.5 RAID controller (raid_ctrl)
`/proc/mdstat` for software RAID. MegaRAID/Dell PERC — deferred-native.

#### 14.6 IPMI/BMC (ipmi)
Device node detection only. Content collection deferred-native.

Path: `/dev/ipmi*` (metadata only).

#### 14.7 GPU management (gpu_mgmt)
DRM sysfs state for all GPUs. Vendor-specific interfaces deferred-native.

Path: `/sys/class/drm/*/`.

---

## Implementation Relationship

This taxonomy defines source ownership and collection boundaries. Per-source
collection decisions are maintained in the
[Coverage Decision Matrix](COVERAGE_DECISION_MATRIX.md). Implementation order is
maintained in the [Implementation Plan](IMPLEMENTATION_PLAN.md).

Every implementation milestone should preserve the same snapshot specification
and manifest vocabulary.

When a new source is proposed, the design is incomplete until:

- the L1 owner and L2 domain are clear;
- the L3 object family has a primary coverage unit;
- safety limits or exclusions are explicit;
- the planner can represent scheduled, limited, skipped, unsupported, and
  not-present outcomes without inventing new vocabulary.
