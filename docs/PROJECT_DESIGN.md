# Project Design

## Purpose

`runfossil` captures Linux runtime state at the moment an incident is detected.
Its primary use case is production recovery: operators often need to restart an
application or host immediately, but that action destroys volatile evidence that
engineering teams need for root-cause analysis.

The tool preserves raw evidence with minimal production impact. It does not try
to replace observability, metrics, tracing, log aggregation, crash reporting, or
application-specific debug tooling. It fills the gap between "the host is broken
right now" and "we still need the runtime scene after recovery."

## Product Positioning

`runfossil` is:

- A root-only Linux runtime capture tool.
- A raw evidence collector.
- A fast and low-disturbance capture path.
- A directory-first evidence package.
- A modular Rust workspace.
- A foundation for offline analysis tools.

`runfossil` is not:

- A live monitoring agent.
- A daemon that continuously samples the system.
- A diagnostic rule engine.
- A replacement for application logs or metrics.
- A Kubernetes control-plane snapshot tool.
- A database, language runtime, or business-data snapshot tool.
- A wrapper around shell commands such as `ss`, `ip`, `lsof`, `systemctl`,
  `journalctl`, `smartctl`, `nvme`, `ipmitool`, or `nvidia-smi`.

## Hard Constraints

### Rust Baseline

The project targets Rust 1.95 as its baseline toolchain. New stable language and
standard-library features may be used when they reduce complexity or remove a
dependency, but the codebase should not use new features only for novelty.

### Safe Project Code

All project-owned Rust crates must forbid unsafe code:

```rust
#![forbid(unsafe_code)]
```

This policy applies to code written in this repository. Third-party crates may
contain internal unsafe code when there is no practical safe alternative, but
those dependencies must be treated as part of the supply-chain risk model and
kept small, justified, pinned, and reviewable.

### Root-Only Execution

The capture command is designed for root. Non-root execution should fail early
with a clear error. This avoids ambiguous partial behavior and gives collectors
access to the runtime state that root is expected to preserve during incidents.

### No External Command Collection

Collectors must not invoke system commands to gather data. Collection must be
implemented through Rust code using Linux runtime interfaces such as virtual
filesystems, netlink, Unix sockets, D-Bus protocol support, journald file or
socket support, and runtime APIs implemented natively in Rust.

### Directory-First Output

The critical capture path writes a snapshot directory first. Packaging and
compression happen only after the raw directory has been persisted. This keeps
incident-time work simple, inspectable, and resilient to interruption.

### No Normal Capture Configuration

The normal capture flow must not depend on user-tuned configuration files or
large flag sets. The tool should derive the best capture plan from host state,
kernel capabilities, runtime pressure, source availability, and estimated cost.

## Design Goals

### Preserve Evidence Before It Disappears

Runtime evidence can disappear when a process exits, a socket closes, a cgroup
is removed, a kernel buffer wraps, a container is deleted, or a host reboots.
The design prioritizes volatile evidence and records failures as evidence rather
than treating them as exceptional tool behavior.

### Keep Production Impact Low

The collector must avoid unbounded recursion, unbounded file reads, expensive
global locks, destructive kernel interfaces, and long blocking operations. Every
capture task must have explicit limits for time, bytes, file count, and depth.

### Prefer Raw Data Over Interpretation

The snapshot should preserve raw files, directory listings, symlink targets,
metadata, netlink responses, event windows, and runtime state records. Parsing
is allowed only when needed to discover more raw objects, control budget, or
write stable metadata.

### Degrade Gracefully

Linux runtime state is unstable by nature. Files disappear, processes exit, and
permissions or kernel policies vary by host. `runfossil` must preserve partial
captures and make missing evidence explicit in the manifest.

### Make Offline Analysis Reproducible

The snapshot must record what was attempted, what succeeded, what failed, why it
failed, what was skipped, and what limits were applied. Later analysis should be
able to reconstruct the capture plan without access to the original host.

## Scope

### Included

- System runtime state.
- CPU, scheduler, interrupt, softirq, memory, VM, slab, PSI, and kernel state.
- Process, thread, file descriptor, address-space, namespace, and cgroup state.
- Network interface, protocol, route, neighbor, socket, conntrack, traffic
  control, and XFRM state where available through native interfaces.
- Block, filesystem, mount, device, driver, module, firmware, power, thermal,
  and hardware-adjacent state exposed by Linux runtime interfaces.
- Kernel ring buffer and bounded system event windows where natively supported.
- Service manager state through native protocol support.
- User, login, session, security, audit, and time synchronization state.
- Crash dump metadata and persistent crash records.
- Local container runtime state through native socket/protocol support.

### Excluded

- Static system configuration as a primary target.
- Application configuration and business data.
- Source code, package manager databases, and container image contents.
- Database internal runtime state.
- JVM, Go, Python, Node.js, .NET, BEAM, or other language-runtime state.
- Application debug endpoints and application metrics systems.
- Kubernetes API and orchestration control-plane state.
- Cloud provider control-plane APIs.
- Full long-term historical logs.

Excluded data may become a separate project or an offline analyzer input, but it
does not belong to the core Linux runtime snapshot boundary.

## Operating Model

The main operator flow is intentionally small:

```text
runfossil capture
runfossil pack <snapshot-dir>
runfossil inspect <snapshot-dir>
```

`capture` creates the raw evidence directory and requires root. `pack` creates a
compressed archive after capture. `inspect` validates and summarizes a snapshot
without requiring the original host or root privileges.

The capture path should remain useful even if only `capture` is implemented.
Packaging and inspection must not be required to preserve evidence.

## Evidence Model

The project uses a three-layer source taxonomy:

```text
L1. Root Collection Source
  L2. Internal Source Domain
    L3. Raw Snapshot Object
```

Runtime execution adds a fourth planning layer:

```text
L1 Source -> L2 Domain -> L3 Raw Snapshot Object -> Capture Task
```

The taxonomy describes what can be captured. The planner decides what should be
captured on this host, under this pressure, within this budget.

## Failure Model

Expected capture outcomes include:

- Captured successfully.
- Source vanished during capture.
- Source not present on this kernel or host.
- Permission denied by kernel policy even as root.
- Timed out.
- Hit byte, file, or depth limits.
- Skipped by planner policy.
- Unsupported by the current implementation.
- Failed with an I/O error.

These outcomes must be represented in the snapshot manifest. Only failures that
make the snapshot directory itself unusable should cause the command to fail
hard.

## Privacy and Sensitivity

Runtime snapshots may contain secrets, credentials, process arguments, process
environment values, file paths, user identities, network peers, tokens, kernel
messages, and container metadata. The tool should not silently redact raw
evidence by default, because redaction can destroy forensic value.

Instead, snapshots should be treated as sensitive artifacts. The design must
support restrictive output permissions, clear metadata, and future offline
redaction tooling.

## Success Criteria

The initial design baseline supports these outcomes:

- A root user can run one capture command during an incident.
- The capture command preserves a useful raw snapshot without external tools.
- The snapshot is understandable as a directory tree.
- The manifest explains every success, failure, skip, and limit.
- The capture plan adapts to host size and pressure without user configuration.
- The codebase can grow collector families independently inside a workspace.
