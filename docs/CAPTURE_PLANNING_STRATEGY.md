# Capture Planning Strategy

## Purpose

The capture planner decides what `runfossil` should capture on the current host
without requiring user configuration. It balances evidence value, runtime cost,
host pressure, and collector risk.

The planner is not a diagnostic engine. It does not decide what caused the
incident. It decides how to preserve the most useful evidence with the least
production impact.

The planner's target is complete accounting of in-scope Linux runtime evidence
within the configured collection budget. It must prefer explicit limited,
skipped, unsupported, `not_present`, `not_found`, failed, or truncated outcomes
over overrunning the budget to make the snapshot appear more complete.

## Planning Principles

- Capture core evidence first.
- Prefer low-cost high-value sources.
- Give every task an explicit budget.
- Adapt to host scale and pressure.
- Deepen collection only when value justifies cost.
- Record why each task was scheduled, limited, skipped, unsupported, or not
  present.
- Never let a single source block the whole snapshot.

## Planning Phases

The baseline planner runs four phases:

```text
probe -> score -> budget -> execute
```

### Probe

Probe gathers low-cost facts needed to build the plan:

- Effective UID and root status.
- Kernel release, boot ID, hostname, architecture.
- Mounted runtime filesystems.
- cgroup version and hierarchy.
- process count, thread count estimate, and PID range.
- CPU count, memory size, block devices, network interfaces.
- PSI pressure where available.
- load average and uptime.
- service manager indicators.
- container daemon socket indicators.
- journald, kmsg, pstore, debugfs, tracefs, bpffs, securityfs availability.

Probe must be bounded. It should not deeply traverse dynamic trees.

### Score

Each candidate task receives:

- Evidence value.
- Estimated cost.
- Risk level.
- Volatility.
- Source availability.
- Dependency on other task results.

Evidence value is high when data is volatile, difficult to reconstruct, or often
important during incidents. Cost is high when data is large, slow, deeply
recursive, or multiplied by many processes, threads, file descriptors, cgroups,
or devices.

### Budget

The planner assigns:

- Global capture budget.
- Per-priority budget.
- Per-source budget.
- Per-task timeout.
- Maximum bytes per object.
- Maximum files per traversal.
- Maximum depth per traversal.
- Maximum concurrency class.

Budgets should be deterministic for similar host states but may adapt to pressure
and scale.

### Execute

The executor runs the selected task graph. Execution feedback can further reduce
work:

- Repeated timeouts reduce similar low-priority tasks.
- High I/O latency reduces deep filesystem traversal.
- Many vanished process paths reduce per-process deepening.
- Size limit hits can stop sibling expansion.
- Source-level permission failures can skip dependent tasks.

Feedback decisions must be recorded in `plan.json` or final task outcomes.

## Plan Decision Vocabulary

The planner uses one decision vocabulary in `plan.json`:

```text
scheduled
limited
skipped_by_policy
unsupported
not_present
```

The terms mean:

- `scheduled`: the task should run with the normal limits assigned to its
  coverage unit and priority.
- `limited`: the task should run, but with stricter limits, selected objects,
  metadata-only capture, reduced recursion, or a smaller event window.
- `skipped_by_policy`: the task should not run because the planner rejected it
  for cost, pressure, risk, release policy, source overlap, or dependency
  failure.
- `unsupported`: the source is valid, but the current implementation has no
  native collector for it.
- `not_present`: probing showed that the source or object is absent on this host.

The final manifest status is recorded separately after execution. A `limited`
plan decision can still produce a `captured` manifest status if the object was
captured within its reduced limits.

## Priority Tiers

### P0: Core Global Runtime Evidence

P0 tasks are small, broadly useful, and volatile. They should run first.

Examples:

```text
/proc/loadavg
/proc/uptime
/proc/stat
/proc/meminfo
/proc/vmstat
/proc/pressure/*
/proc/diskstats
/proc/partitions
/proc/mounts
/proc/self/mountinfo
/proc/net/dev
/proc/net/snmp
/proc/net/netstat
/proc/modules
/proc/interrupts
/proc/softirqs
kernel ring buffer window
basic host metadata
```

### P1: Process Summary Evidence

P1 captures enough process state to understand what was running and which
processes deserve deeper collection.

Examples:

```text
/proc/<pid>/status
/proc/<pid>/stat
/proc/<pid>/statm
/proc/<pid>/cmdline
/proc/<pid>/comm
/proc/<pid>/cgroup
/proc/<pid>/io
/proc/<pid>/limits
/proc/<pid>/sched
/proc/<pid>/oom_score
/proc/<pid>/oom_score_adj
/proc/<pid>/ns symlinks
/proc/<pid>/fd listing
```

### P2: Subsystem Runtime Evidence

P2 captures major kernel and device subsystems.

Examples:

```text
/sys/class/net
/sys/block
/sys/fs/cgroup
/sys/devices/system/cpu
/sys/devices/system/node
/sys/module
/sys/fs/pstore
/sys/class/hwmon
/sys/class/thermal
/proc/<pid>/smaps_rollup
netlink link/address/route/neighbor dumps
```

### P3: Conditional Deep Evidence

P3 tasks are valuable but can be expensive or multiplied by scale.

Examples:

```text
/proc/<pid>/fdinfo
/proc/<pid>/maps
/proc/<pid>/task
/proc/<pid>/task/<tid>/status
service manager unit state
container-related cgroup and namespace evidence
socket diagnostic netlink dumps
cgroup subtree deep traversal
bounded system event windows
```

### P4: High-Cost or High-Risk Evidence

P4 tasks are disabled, heavily limited, or conditionally selected unless strong
signals justify them.

Examples:

```text
/proc/<pid>/smaps
/proc/<pid>/pagemap
/proc/<pid>/stack
/proc/kcore metadata only
debugfs deep traversal
tracefs buffers
large bpffs dumps
large /run runtime trees
large device tree traversal
```

P4 does not mean "never collect"; it means "do not let this interfere with
preserving core evidence."

## Dynamic Signals

### Host Scale

The planner should adapt to:

- CPU count.
- Memory size.
- Number of processes.
- Estimated number of threads.
- Number of open file descriptors.
- Number of cgroups.
- Number of mounts.
- Number of network interfaces.
- Number of block devices.
- Size of `/sys` and `/run` candidate trees.

Large hosts should receive stricter per-object and per-tree budgets before deep
collection begins.

### Runtime Pressure

Pressure signals include:

- PSI CPU, memory, and I/O values.
- Load average relative to CPU count.
- Memory availability.
- Swap activity.
- Block I/O counters.
- Scheduler blocked process counters.
- OOM counters or OOM messages.
- Kernel ring buffer errors.

High pressure should not disable capture. It should shift capture toward
smaller, more volatile, higher-value evidence.

### Incident Signals

Signals that justify deeper collection:

- OOM or allocation failure messages.
- cgroup OOM events.
- Hung task, soft lockup, hard lockup, or RCU stall messages.
- Filesystem errors or read-only remounts.
- Block device errors.
- Network driver resets or link flaps.
- Service failures or degraded service manager state.
- Container OOM, restart, or crash indicators.
- High memory pressure with suspicious RSS or OOM scores.
- High I/O pressure with blocked processes.

The planner should deepen the relevant source family rather than globally
switching into a heavy mode.

## Budget Model

Every task has:

```text
priority
risk
source budget
task timeout
max bytes
max files
max depth
concurrency class
```

Risk levels:

```text
low
medium
high_cost
policy_sensitive
unstable
```

The budget model should be visible in `plan.json` and `meta/limits.json`.

### Budget Invariant

Configured budgets are hard ceilings for incident-time capture. The planner and
executor may reduce depth, file count, byte windows, concurrency, or lower
priority work when host scale, pressure, or execution feedback indicates that
the full candidate set will not fit.

Budget exhaustion is an evidence outcome, not an excuse to overrun. When the
budget prevents additional capture, the plan and manifest should preserve the
reason through `limited`, `skipped_by_policy`, `size_limited`, `truncated`,
`timeout`, or related outcomes.

## Source-Level Strategies

### /proc

Strategy:

- Always capture core global files.
- Always capture process summaries.
- Deepen per-process state based on scale and incident signals.
- Treat vanished processes as normal.
- Avoid unbounded per-thread and per-fd expansion on very large systems.

### /sys

Strategy:

- Capture major class and subsystem state with depth limits.
- Prefer known high-value subtrees over full recursive `/sys` traversal.
- Use stricter limits for debugfs, tracefs, bpffs, and device trees.
- Capture pstore early if present because it may contain crash evidence.

### /run

Strategy:

- Capture runtime metadata and socket presence.
- Avoid copying large runtime payload trees blindly.
- Prioritize systemd, user session, udev, D-Bus, resolver, and runtime socket
  presence metadata.

### /dev

Strategy:

- Capture metadata, device node listings, symlink targets, and major/minor
  numbers.
- Do not read streaming or destructive device contents.
- Treat pseudo devices as metadata targets, not byte-stream targets.

### Kernel and Logs

Strategy:

- Capture bounded kernel ring buffer windows.
- Capture system event windows only through native implementations.
- Avoid full historical logs by default.
- Expand windows when incident signals are present.

### Netlink

Strategy:

- Use native netlink for structured runtime state.
- Capture link, address, route, neighbor, and socket diagnostic state when
  available.
- Fall back to raw `/proc/net` where netlink family support is missing.

### Service Manager

Strategy:

- Detect service manager presence.
- Capture manager and unit state through native protocol support.
- Avoid external commands.
- Record unsupported state clearly until native collection exists.

### Container-Related Host Evidence

Strategy:

- Detect local runtime sockets as host probes and capability metadata.
- Capture container-related process, cgroup, namespace, and resource evidence
  through the existing `/proc`, `/sys/fs/cgroup`, and `/run` source families.
- Do not enumerate Docker, containerd, CRI-O, or other daemon APIs in the core
  scope unless the Source Taxonomy and Coverage Decision Matrix change first.

## Skip Policy

Skipping must be explicit. Valid skip reasons include:

- Source not present.
- Collector not implemented.
- Host pressure too high for the task's cost.
- Parent task failed.
- Equivalent higher-value source already captured.
- Task exceeds scale threshold.
- Task requires a risky interface not enabled in the initial release.

Skipped objects should appear in `plan.json`; important skipped objects should
also appear in the manifest.

Skip terminology:

- Use `not_present` when a source or object is absent on this host.
- Use `unsupported` when the source is valid but no native collector exists.
- Use `skipped_by_policy` when the planner could collect the object in principle
  but chooses not to under current policy, cost, risk, or pressure.

## Determinism

Planner behavior should be stable for similar host states. This matters for
testing and operator trust. Dynamic adaptation is allowed, but it should be based
on recorded probes and deterministic thresholds.

## Future Planner Enhancements

Potential later improvements:

- Learned default budgets from real capture benchmarks.
- Offline replay of planner decisions from captured metadata.
- More precise process suspicion scoring.
- Optional operator-provided capture goal without exposing a large config file.
- Policy bundles for regulated environments.
