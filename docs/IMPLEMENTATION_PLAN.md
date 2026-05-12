# Implementation Plan

## Planning Principles

The project should evolve by preserving stable contracts first:

- Snapshot directory format.
- Manifest status vocabulary.
- Source coverage decision matrix.
- Planner decision records.
- Safe project-owned Rust code.
- Root-only capture.
- No external command collection.

Implementation can then expand source coverage without invalidating earlier
snapshots.

The project is designed for AI-driven development. AI coding agents are expected
to contribute implementation work guided by `AGENTS.md` and the design baseline
documents. This implementation plan provides the delivery order; AI skill specs
and prompt templates provide the workflow. The `.codex/` and `.agents/`
directories are reserved for tool-specific implementation configuration.

## Milestone 0: Design Baseline

Status: complete.

Deliverables:

- Project design.
- Architecture document.
- Snapshot specification.
- Capture planning strategy.
- Source Taxonomy.
- Coverage Decision Matrix.
- Requirements Traceability.
- Glossary.
- Risk Register.
- Design Review Checklist.
- Safety and Operations policy.
- Architectural Decision Record.
- Implementation plan.
- AI agent instructions.
- AI context index, development guide, skill specs, and prompt library.
- Documentation Readiness Gate.
- Documentation Readiness Review.

Exit criteria:

- The repository documents product goals and non-goals.
- The snapshot format and planner behavior are clear enough to implement.
- The implementation constraints are explicit.
- Coverage decisions, plan decisions, and manifest statuses have an explicit
  vocabulary mapping.
- AI-assisted development has canonical repository instructions and context
  routing.
- The Documentation Readiness Gate and Design Review Checklist pass with no
  blocker.
- The Documentation Readiness Review records the passing result.

## Documentation-First Gate

Milestone 1 must not begin until Milestone 0 passes the documentation-first
gate. Passing means:

- all non-code project documents are current and linked from README;
- scope, safety, snapshot, coverage, planner, requirements, risk, ADR, glossary,
  implementation order, and AI workflow documents are mutually consistent;
- every explicit requirement has traceability and acceptance criteria;
- every known material risk has a mitigation or accepted status;
- no document claims implementation behavior that does not exist yet;
- documentation-only validation passes.

If implementation work reveals a design gap later, the project returns to this
gate for the affected documents before accepting the new behavior.

## Milestone 1: Workspace Skeleton

Status: complete.

Deliverables:

- Rust 1.95 workspace with all planned crates.
- `runfossil-cli` (binary entry point with capture, pack, inspect subcommands).
- `runfossil-core` (shared domain model and invariants).
- `runfossil-store` (snapshot directory creation and persistence).
- `runfossil-plan` (planner model with registry, builder, host probe).
- `runfossil-fs` (bounded filesystem helpers).
- `runfossil-proc` (procfs collector with P0/P1/P2/P3 coverage).
- `runfossil-net` (network and netlink collector).
- `runfossil-service` (service manager collector with D-Bus protocol).
- `runfossil-container` (container runtime collector with host-side detection).
- `runfossil-pack` (post-capture packaging and inspection).
- Project-wide `#![forbid(unsafe_code)]` on every crate.
- Bounded concurrent executor with scoped threads and cancel tokens.
- SIGINT/SIGTERM signal handling via isolated `runfossil-signal` crate.
- Formatting, lint, build, test, and benchmark commands.
- CI/CD workflow with check, test, clippy, fmt, and cargo-deny audit.
- Stress tests and criterion benchmarks.

Exit criteria (all met):

- Workspace builds on Linux.
- Non-root capture fails early.
- A minimal capture creates a valid snapshot directory.
- `CAPTURE_COMPLETE` is written only on successful finalization.
- Project has no `unsafe_code` outside the signal sandbox crate.
- No collector executes external commands.
- Coverage decisions, plan decisions, and manifest statuses remain separate vocabularies.

## Milestone 2: Snapshot Store and Manifest

Status: complete.

Deliverables:

- Directory creation.
- Atomic file writes.
- `manifest.json`.
- `plan.json`.
- `errors.jsonl`.
- Metadata files.
- Object status vocabulary.
- Partial snapshot behavior.

Exit criteria (all met):

- Interrupted captures leave inspectable partial directories.
- Manifest records success, failure, skipped, unsupported, timeout, and limit outcomes.
- Snapshot directory permissions are restrictive.

## Milestone 3: Core /proc and /sys Capture

Status: complete.

Deliverables:

- P0 `/proc` global files (loadavg, meminfo, vmstat, stat, cpuinfo, etc.).
- P1 process summaries (status, stat, cmdline, environ, ns, fd, limits, io, sched).
- Basic thread and fd listing (fd, fdinfo, task).
- Core `/sys` class and subsystem capture (class/net, block, devices, power, cgroup).
- Cgroup v1/v2 detection and bounded capture.
- pstore metadata and records where present.

Exit criteria (all met):

- Capture is useful on a standard Linux host without external tools.
- Process disappearance races are handled cleanly.
- Large host scale triggers budget limits instead of unbounded traversal.

## Milestone 4: Adaptive Planner

Status: complete.

Deliverables:

- Host probe model (loadavg, pressure, meminfo, stat, scale estimation).
- Pressure and scale estimation.
- Priority tiers (P0-P4).
- Source and task budgets.
- Conditional deepening rules.
- Deterministic plan records.

Exit criteria (all met):

- Similar probe inputs produce stable plans.
- High pressure reduces deep work.
- Incident signals deepen relevant source families.
- `plan.json` explains scheduling and skip decisions.

## Milestone 5: Kernel, Network, and Device State

Status: complete.

Deliverables:

- Kernel ring buffer bounded windows.
- `/proc/net` raw capture (tcp, udp, unix, dev, route, arp, snmp, netstat).
- Native netlink link, address, route, and neighbor capture.
- `/dev` metadata and symlink capture (block, mapper, loop, pseudo devices).
- Block and network device state.

Exit criteria (all met):

- Network and device state can be preserved without `ip`, `ss`, `tc`, or `lsof`.
- Device nodes are treated as metadata unless explicitly safe to read.

## Milestone 6: Service, Session, Time, and Logs

Status: implemented.

Deliverables:

- Native service manager detection.
- systemd manager and unit state through custom D-Bus protocol implementation.
- logind and session runtime state through D-Bus.
- bounded system event windows through filesystem collection.
- time synchronization runtime state where supported (uptime, localtime, timesync).

Exit criteria:

- No service or log command execution is used.
- Unsupported service features are recorded clearly.
- Native D-Bus implementation replaces `systemctl` and `journalctl`.

## Milestone 7: Container Runtime State

Status: host-side evidence implemented; native protocol deferred.

Deliverables:

- Runtime socket detection implemented (docker, containerd, crio, runc).
- Host-side cgroup and namespace container evidence implemented.
- Docker/containerd/CRI-O native protocol support remains deferred-native.

Exit criteria (host-side met, protocol remains planned):

- Host-side container evidence is available without invoking container CLIs.
- Container state is collected from procfs, cgroupfs, and namespace evidence.

## Milestone 8: Packaging and Inspection

Status: complete.

Deliverables:

- `runfossil pack <snapshot-dir>` producing `.tar.zst` archives.
- Archive integrity metadata (SHA-256, compression format, size).
- `runfossil inspect <snapshot-dir>` offline validation.
- Snapshot completeness checks.
- Manifest and plan validation.

Exit criteria (all met):

- Packaging preserves raw evidence exactly.
- Inspection can summarize complete and partial snapshots offline.

## Milestone 9: Hardening

Status: in progress.

Deliverables:

- Stress tests for bounded limits (10k manifest entries, concurrent writes, large snapshots).

- Rust 1.95 workspace with all planned crates.
- `runfossil-cli` (binary entry point).
- `runfossil-core` (shared domain model and invariants).
- `runfossil-store` (snapshot directory creation and persistence).
- `runfossil-plan` (planner model skeleton).
- `runfossil-fs` (bounded filesystem helpers).
- `runfossil-proc` (procfs collector skeleton).
- `runfossil-net` (network and netlink collector skeleton).
- `runfossil-service` (service manager collector skeleton).
- `runfossil-container` (container runtime collector skeleton).
- `runfossil-pack` (post-capture packaging skeleton).
- Project-wide `#![forbid(unsafe_code)]` on every crate.
- Formatting, lint, build, and test commands.

Exit criteria:

- Workspace builds on Linux.
- Non-root capture fails early.
- A minimal capture creates a valid snapshot directory.
- `CAPTURE_COMPLETE` is written only on successful finalization.

## Milestone 2: Snapshot Store and Manifest

Deliverables:

- Directory creation.
- Atomic file writes.
- `manifest.json`.
- `plan.json`.
- `errors.jsonl`.
- Metadata files.
- Object status vocabulary.
- Partial snapshot behavior.

Exit criteria:

- Interrupted captures leave inspectable partial directories.
- Manifest records success, failure, skipped, unsupported, timeout, and limit
  outcomes.
- Snapshot directory permissions are restrictive.

## Milestone 3: Core /proc and /sys Capture

Deliverables:

- P0 `/proc` global files.
- P1 process summaries.
- Basic thread and fd listing.
- Core `/sys` class and subsystem capture.
- Cgroup v1/v2 detection and bounded capture.
- pstore metadata and records where present.

Exit criteria:

- Capture is useful on a standard Linux host without external tools.
- Process disappearance races are handled cleanly.
- Large host scale triggers budget limits instead of unbounded traversal.

## Milestone 4: Adaptive Planner

Deliverables:

- Host probe model.
- Pressure and scale estimation.
- Priority tiers.
- Source and task budgets.
- Conditional deepening rules.
- Deterministic plan records.

Exit criteria:

- Similar probe inputs produce stable plans.
- High pressure reduces deep work.
- Incident signals deepen relevant source families.
- `plan.json` explains scheduling and skip decisions.

## Milestone 5: Kernel, Network, and Device State

Deliverables:

- Kernel ring buffer bounded windows.
- `/proc/net` raw capture.
- Native netlink link, address, route, and neighbor capture.
- Socket diagnostic capture where feasible.
- `/dev` metadata and symlink capture.
- Block and network device state.

Exit criteria:

- Network and device state can be preserved without `ip`, `ss`, `tc`, or `lsof`.
- Device nodes are treated as metadata unless explicitly safe to read.

## Milestone 6: Service, Session, Time, and Logs

Deliverables:

- Native service manager detection.
- systemd manager and unit state through native protocol support.
- logind and session runtime state where supported.
- bounded system event windows through native implementation.
- time synchronization runtime state where supported.

Exit criteria:

- No service or log command execution is used.
- Unsupported service features are recorded clearly.

## Milestone 7: Container Runtime State

Deliverables:

- Runtime socket detection.
- Native Docker/containerd/CRI-O support where feasible.
- Container object, process, cgroup, namespace, and resource evidence.
- Bounded container event and log windows.

Exit criteria:

- Host-side container evidence is available even before all runtime protocols are
  complete.
- Container state is collected without invoking container CLIs.

## Milestone 8: Packaging and Inspection

Deliverables:

- `runfossil pack <snapshot-dir>`.
- Archive integrity metadata.
- `runfossil inspect <snapshot-dir>`.
- Snapshot completeness checks.
- Manifest and plan validation.

Exit criteria:

- Packaging preserves raw evidence exactly.
- Inspection can summarize complete and partial snapshots offline.

## Milestone 9: Hardening

Status: in progress.

Deliverables:

- Stress tests for bounded limits (10k manifest entries, concurrent writes, large snapshots).
- Benchmark suite for store and pack operations (criterion).
- CI/CD pipeline with dependency caching and cargo-deny audit.
- Release profile optimizations (LTO, single codegen-unit, panic=abort, strip).
- Signal handling integration (SIGINT/SIGTERM with graceful cancellation).
- Coverage registry updated to reflect implemented native protocol support.

Exit criteria:

- Capture remains bounded on large hosts.
- No project-owned unsafe code exists outside the signal sandbox crate.
- No collector executes external commands.
- Snapshot format remains backward-compatible.
- Benchmarks exist for performance-sensitive operations.
- Stress tests verify bounded behavior under load.

## Resolved Design Defaults

These defaults keep implementation work bounded without reopening product scope:

- The snapshot directory is the canonical artifact. `pack` should default to a
  post-capture `.tar.zst` archive created through project code or reviewed Rust
  libraries, not through shelling out to `tar` or `zstd`.
- Hashes should be computed for manifest/control files and for captured
  file-like payloads when doing so does not delay incident-time capture. Skipped
  hashes must be explicit in manifest object metadata.
- Native journald or system log support is deferred until dependency and parser
  risk are reviewed. Kernel ring buffer capture remains the primary early log
  source.
- Initial netlink scope is link, address, route, and neighbor state. Socket
  diagnostic, conntrack, traffic control, and XFRM state remain conditional.
- Container delivery starts with host-side procfs, namespace, and cgroup evidence,
  then adds native runtime protocols incrementally.
- Hardware management beyond Linux-exposed `/sys` state is deferred until core
  filesystem, procfs, netlink, and snapshot-store behavior is stable.
