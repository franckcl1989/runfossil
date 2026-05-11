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

## Milestone 0: Design Baseline

Status: current.

Deliverables:

- Project design.
- Architecture document.
- Snapshot specification.
- Capture planning strategy.
- Source Taxonomy.
- Coverage Decision Matrix.
- Requirements Traceability Matrix.
- Glossary.
- Risk Register.
- Design Review Checklist.
- Safety and Operations policy.
- Implementation plan.

Exit criteria:

- The repository documents product goals and non-goals.
- The snapshot format and planner behavior are clear enough to implement.
- The implementation constraints are explicit.

## Milestone 1: Workspace Skeleton

Deliverables:

- Rust 1.95 workspace.
- CLI crate.
- Core model crate.
- Store crate.
- Planner crate.
- Initial filesystem collector crate.
- Project-wide `#![forbid(unsafe_code)]`.
- Formatting, lint, and test commands.

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

Deliverables:

- Expanded tests.
- Stress fixtures for large process and cgroup counts.
- Permission and race-condition tests.
- Dependency review.
- Benchmark captures on representative hosts.
- Documentation updates from implementation findings.

Exit criteria:

- Capture remains bounded on large hosts.
- No project-owned unsafe code exists.
- No collector executes external commands.
- Snapshot format remains backward-compatible.

## Open Design Questions

These should be answered during implementation:

- Exact archive compression format for `pack`.
- Whether hashes are required for all captured payloads or only small payloads.
- How much journald support can be implemented natively without excessive
  dependency risk.
- Which netlink families are part of the initial release versus later releases.
- Which container runtime protocols should be implemented first.
- Whether hardware management beyond `/sys` should wait until after core
  collectors are stable.
