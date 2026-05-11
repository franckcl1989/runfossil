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

## L1 Source Families

The project recognizes these top-level source families:

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
15. container runtime
```

Subpaths such as `/proc/net`, `/proc/<pid>`, `/sys/fs/cgroup`,
`/sys/class/net`, `/run/systemd`, and `/sys/fs/pstore` remain L2 domains under
their root source. They are not promoted to L1.

## 1. /proc

Purpose:

- Primary kernel and process runtime state.
- System counters, memory, scheduler, network protocol views, IPC, mounts,
  kernel parameters, modules, and process state.

Important domains:

- System overview.
- CPU and scheduler.
- Memory and virtual memory.
- PSI pressure.
- Interrupt and softirq.
- Block device statistics.
- Network protocol stack.
- IPC.
- File locks.
- Mount and filesystem view.
- Kernel parameters.
- Kernel modules.
- Kernel resource and hardware maps.
- Crypto, key, and timer state.
- Process and thread state.

Collection policy:

- P0 for global low-cost files.
- P1 for process summaries.
- P3/P4 for deep process data such as `smaps`, `pagemap`, kernel stacks, and
  thread expansion.
- Process disappearance is expected and recorded as `vanished`.

## 2. /sys

Purpose:

- Kernel object model, devices, drivers, classes, cgroups, tracing filesystems,
  security filesystems, firmware state, pstore, and subsystem runtime state.

Important domains:

- Network devices.
- Block devices.
- CPU devices.
- NUMA.
- Cgroups.
- Kernel modules.
- Kernel state.
- DebugFS.
- TraceFS.
- BPF filesystem.
- SecurityFS.
- ConfigFS.
- Pstore.
- Filesystem state.
- Firmware.
- Power.
- Hardware monitor and thermal.
- Graphics and accelerator devices.
- RDMA and InfiniBand.
- NVMe and storage classes.
- Watchdog, RTC, input, and miscellaneous classes.
- Hypervisor and virtualization.
- IOMMU.
- Bus, device, and driver trees.

Collection policy:

- Capture known high-value subtrees with depth and file limits.
- Avoid blind full `/sys` recursion.
- Treat debugfs, tracefs, and bpffs as high-cost or policy-sensitive.
- Capture pstore early when present.

## 3. /run

Purpose:

- Boot-lifetime runtime state created by services and daemons.

Important domains:

- systemd runtime.
- User runtime.
- Locks.
- Daemon PID files.
- Container runtime temporary state.
- Udev runtime.
- D-Bus runtime.
- Resolver runtime.

Collection policy:

- Capture metadata, socket presence, small runtime files, and known state
  locations.
- Avoid copying large or unbounded runtime directories blindly.
- Prefer native protocol collectors for service and container details.

## 4. /dev

Purpose:

- Runtime device namespace and device node metadata.

Important domains:

- Device nodes.
- Block device nodes.
- Device mapper.
- Loop devices.
- FUSE.
- TTY and PTY.
- Randomness and kernel message devices.
- Pseudo devices.

Collection policy:

- Capture directory listings, device metadata, symlink targets, major/minor
  numbers, and permissions.
- Do not read from streaming, blocking, destructive, or infinite devices.
- `/dev/kmsg` belongs to the kernel ring buffer collector when used as an event
  source.

## 5. Kernel Ring Buffer

Purpose:

- Volatile kernel event evidence that may be overwritten or lost after reboot.

Important domains:

- Kernel error events.
- OOM events.
- Lockup and stall events.
- Device and driver events.
- Filesystem events.
- Network kernel events.

Collection policy:

- Capture bounded event windows.
- Expand relevant windows when incident signals appear.
- Avoid destructive reads that clear evidence unless the kernel interface and
  implementation guarantee non-destructive behavior.

## 6. System Event Log Store

Purpose:

- Bounded runtime event windows from local logging facilities.

Important domains:

- System event windows.
- Kernel event mirrors.
- Service event windows.
- Authentication and security events.
- Container event windows.
- Audit event windows.

Collection policy:

- Native implementation only.
- No `journalctl` or log command invocation.
- No full long-term historical log capture by default.
- Store bounded windows and source metadata.

## 7. Service Manager

Purpose:

- Runtime state maintained by the local init or service management system.

Important domains:

- Manager state.
- Unit state.
- Service state.
- Socket unit state.
- Timer unit state.
- Failed or degraded state.

Collection policy:

- systemd is the primary target.
- Native protocol support is required.
- Until implemented, record service manager availability and unsupported details
  rather than invoking commands.

## 8. Netlink

Purpose:

- Native structured runtime state for networking and selected kernel subsystems.

Important domains:

- Link.
- Address.
- Route.
- Neighbor.
- Socket diagnostic.
- Conntrack.
- Traffic control.
- XFRM.

Collection policy:

- Prefer netlink for data that is more complete or reliable than procfs.
- Preserve raw responses or stable serialized records.
- Use `/proc/net` as supplemental raw evidence.

## 9. Security / Audit Subsystem

Purpose:

- Runtime state from Linux security modules, audit, and privilege-related
  subsystems.

Important domains:

- Process security.
- SELinux.
- AppArmor.
- Audit.
- Privilege events.

Collection policy:

- Capture process security state visible through procfs and securityfs.
- Capture audit state only through native support.
- Treat policy-denied sources as normal manifest outcomes.

## 10. User / Session Database

Purpose:

- Logged-in users, sessions, terminals, seats, and interactive process
  relationships.

Important domains:

- Login sessions.
- SSH sessions.
- TTY and PTY.
- logind.
- Interactive process relations.

Collection policy:

- Capture runtime files and native protocol state where available.
- Avoid shelling out to `who`, `w`, `loginctl`, or similar tools.

## 11. Scheduler / Job Runtime

Purpose:

- Runtime state of local job schedulers, not static schedule configuration.

Important domains:

- Cron runtime.
- systemd timer runtime.
- at and batch runtime.
- External job runtime.

Collection policy:

- Capture running or recent job state when exposed through runtime files or
  native protocols.
- Do not collect static crontabs as a primary target.

## 12. Time Synchronization Subsystem

Purpose:

- Runtime state of system clocks and synchronization services.

Important domains:

- System clock.
- NTP.
- chrony.
- systemd-timesyncd.
- PTP.

Collection policy:

- Capture local clock metadata.
- Capture service state natively when supported.
- Avoid invoking `chronyc`, `timedatectl`, or similar commands.

## 13. Crash Dump Store

Purpose:

- Runtime crash metadata and persisted crash artifacts.

Important domains:

- User-space core dump metadata.
- systemd-coredump metadata.
- Kernel crash dump metadata.
- Recent crash records.

Collection policy:

- Capture metadata and bounded records first.
- Do not copy large core or vmcore payloads by default.
- Preserve enough references for later manual retrieval.

## 14. Hardware Management Interface

Purpose:

- Runtime hardware state exposed outside ordinary `/sys` paths by management
  controllers, device-specific interfaces, or vendor protocols.

Important domains:

- Disk health.
- NVMe management.
- RAID controller state.
- IPMI, BMC, and Redfish state.
- GPU management.
- RDMA and InfiniBand.
- Sensors, thermal, and power.

Collection policy:

- Prefer Linux-exposed runtime state under `/sys` and native kernel interfaces.
- Do not invoke vendor CLI tools.
- Do not depend on C SDK wrappers in project-owned code.
- Mark unavailable hardware management features unsupported until native Rust
  access is designed.

## 15. Container Runtime

Purpose:

- Local container runtime state, excluding Kubernetes control-plane state.

Important domains:

- Runtime daemon.
- Container objects.
- Container processes.
- Container resources.
- Container namespaces.
- Container cgroups.
- Container events and bounded logs.

Collection policy:

- Detect runtime sockets.
- Use native socket protocols when implemented.
- Do not invoke container CLIs.
- Capture host-side process, namespace, and cgroup evidence even before full
  runtime protocol support exists.

## Implementation Relationship

This taxonomy defines source ownership and collection boundaries. Per-source
collection decisions are maintained in the
[Coverage Decision Matrix](COVERAGE_DECISION_MATRIX.md). Implementation order is
maintained in the [Implementation Plan](IMPLEMENTATION_PLAN.md) so the project
has one authoritative delivery plan.

Every implementation milestone should preserve the same snapshot specification
and manifest vocabulary.

When a new source is proposed, the design is incomplete until:

- the L1 owner and L2 domain are clear;
- the L3 object family has a primary coverage unit;
- safety limits or exclusions are explicit;
- the planner can represent scheduled, limited, skipped, unsupported, and
  not-present outcomes without inventing new vocabulary.
