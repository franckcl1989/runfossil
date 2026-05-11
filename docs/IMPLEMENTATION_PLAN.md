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

Status: in progress.

Deliverables:

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
