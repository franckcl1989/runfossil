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

- Rust 1.95 workspace with planned crates.
- `runfossil-cli` binary entry point.
- `runfossil-core` shared domain model and invariants.
- `runfossil-store` snapshot directory and manifest foundation.
- `runfossil-plan` planner model foundation.
- `runfossil-fs` bounded filesystem helper foundation.
- `runfossil-proc`, `runfossil-net`, `runfossil-service`, and
  `runfossil-container` collector crate boundaries.
- `runfossil-pack` post-capture packaging and inspection crate boundary.
- Project-wide `#![forbid(unsafe_code)]` on every project-owned Rust crate.
- Formatting, lint, build, and test commands.

Exit criteria:

- Workspace builds on Linux.
- Non-root capture fails early.
- A minimal capture creates a valid snapshot directory.
- `CAPTURE_COMPLETE` is written only on successful finalization.
- Project-owned Rust code contains no unsafe code.
- No collector executes external commands.
- Coverage decisions, plan decisions, and manifest statuses remain separate
  vocabularies.

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

Exit criteria:

- Interrupted captures leave inspectable partial directories.
- Manifest records success, failure, skipped, unsupported, timeout, and limit
  outcomes.
- Snapshot directory permissions are restrictive.

## Milestone 3: Core /proc and /sys Capture

Status: complete.

Deliverables:

- P0 `/proc` global files.
- P1 process summaries.
- Basic thread and fd listing.
- Core `/sys` class and subsystem capture.
- Cgroup v1/v2 detection and bounded capture.
- Pstore metadata and records where present.

Exit criteria:

- Capture is useful on a standard Linux host without external tools.
- Process disappearance races are handled cleanly.
- Large host scale triggers budget limits instead of unbounded traversal.

## Milestone 4: Adaptive Planner

Status: complete.

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

Status: complete.

Deliverables:

- Kernel ring buffer bounded windows.
- `/proc/net` raw capture.
- Native netlink link, address, route, and neighbor capture.
- `/dev` metadata and symlink capture.
- Block and network device state.

Exit criteria:

- Network and device state can be preserved without `ip`, `ss`, `tc`, or
  `lsof`.
- Device nodes are treated as metadata unless explicitly safe to read.

## Milestone 6: Service, Session, Time, and Logs

Status: implemented; hardening continues in Milestone 9.

Deliverables:

- Native service manager detection.
- systemd manager and unit state through custom D-Bus protocol implementation.
- logind and session runtime state through D-Bus.
- Bounded system event windows through filesystem collection where supported.
- Time synchronization runtime state where supported.

Exit criteria:

- No service or log command execution is used.
- Unsupported service features are recorded clearly.
- Native D-Bus implementation replaces `systemctl`, `loginctl`, and
  `journalctl` for in-scope service/session capture.

## Milestone 7: Container Runtime State

Status: host-side evidence implemented; daemon API protocols excluded from the
current kernel-focused scope.

Deliverables:

- Runtime socket detection.
- Host-side cgroup and namespace container evidence.
- Container process, resource, namespace, and cgroup evidence from procfs and
  cgroupfs.

Exit criteria:

- Host-side container evidence is available without invoking container CLIs.
- Container state is collected from procfs, cgroupfs, and namespace evidence.
- Docker, containerd, and CRI-O daemon API enumeration remains out of core scope
  unless the coverage matrix is changed first.

## Milestone 8: Packaging and Inspection

Status: complete.

Deliverables:

- `runfossil pack <snapshot-dir>` producing `.tar.zst` archives.
- Archive integrity metadata.
- `runfossil inspect <snapshot-dir>` offline validation.
- Snapshot completeness checks.
- Manifest and plan validation.

Exit criteria:

- Packaging preserves raw evidence exactly.
- Inspection can summarize complete and partial snapshots offline.

## Milestone 9: Hardening and Release Readiness

Status: automated gates passing; production validation pending manual execution.

Deliverables:

- Stress tests for bounded limits, large manifests, concurrent writes, and large
  snapshots. **[implemented]**
- Benchmark suite for store and pack operations. **[implemented]**
- CI pipeline with check, test, clippy, fmt, release check, and cargo-deny audit.
  **[implemented]**
- Release profile optimizations for a small single binary. **[implemented]**
- Coverage registry alignment with implemented native and filesystem-backed
  collectors. **[implemented]**
- Documentation-to-code consistency audit before release. **[completed]**
- Production-readiness validation report covering safety, performance,
  compatibility, and known limitations. **[automated gates pass; production validation
  requires privileged execution environments]**
- Safe SIGINT/SIGTERM cancellation through a reviewed third-party signal
  abstraction, without project-owned unsafe code. **[implemented via nix crate]**

Exit criteria:

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo check --workspace --all-targets`, and `cargo test --workspace` pass
  locally and in CI. **[passing locally; CI status per latest run]**
- Benchmarks compile under `cargo check --workspace --all-targets`; performance
  runs are executed separately from the default test gate. **[benchmarks compile]**
- Capture remains bounded on large hosts. **[bounded by global timeout, per-task
  limits, and traversal limits]**
- Project-owned Rust code contains no unsafe code. **[verified: 34 source files
  with `#![forbid(unsafe_code)]`]**
- No collector executes external commands. **[verified: zero `std::process::Command`
  usage; enforced by clippy.toml]**
- Snapshot format remains backward-compatible. **[schema_version preserved; path
  rules and vocabulary unchanged]**
- Benchmarks exist for performance-sensitive operations. **[store and pack benches
  in place]**
- Stress tests verify bounded behavior under load. **[9 stress tests passing]**
- Static release artifacts are built for `x86_64-unknown-linux-musl` and
  verified to have no dynamic runtime dependencies. **[build script and CI job
  implemented]**
- Release artifacts have checksum and signature verification records. **[sha256
  generated; signing key held externally]**
- Root production-like capture, cross-distribution compatibility, and benchmark
  runs are recorded in the release readiness report. **[pending manual execution
  in privileged environments]**
- README, AGENTS, AI docs, implementation plan, glossary, requirements, risks,
  and code comments do not claim stale milestone state. **[verified]**
- Release notes distinguish implemented behavior, exclusions, deferred work, and
  validation actually run. **[pending]**

The automated hardening gates are complete. Remaining work requires privileged
Linux execution environments (root capture, cross-distribution testing,
benchmark runs on release hardware). These are recorded as pending in
[Release Readiness](RELEASE_READINESS.md).

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
- Container delivery is host-side procfs, namespace, and cgroup evidence unless
  the coverage matrix later accepts native daemon protocols.
- Hardware management beyond Linux-exposed `/sys` state is deferred until core
  filesystem, procfs, netlink, and snapshot-store behavior is stable.
- Signal handling must not reintroduce project-owned unsafe code. Graceful
  cancellation on SIGINT/SIGTERM uses a reviewed safe third-party abstraction;
  dependency risk is tracked in the Risk Register.

## Planner-Executor Relationship

The adaptive planner produces `plan.json` with per-coverage-unit scheduling
decisions (scheduled, limited, skipped_by_policy, not_present, unsupported).
The `CapturePlanProbes` field records host facts used to derive those decisions.

The capture executor uses a hardcoded task list that maps each L1 source family
to a single collector function. Each task runs unconditionally within its
per-task timeout and global byte/time budgets. The executor does not consult
per-coverage-unit plan decisions at runtime.

This design is an intentional simplification for the baseline release:

- **plan.json is an offline reference**. It records what the planner *would*
  schedule on this host. Later analysis tools can compare planned coverage
  against actual manifest outcomes without requiring the original host.
- **Task execution is bounded anyway**. The per-task timeout, global timeout,
  per-file byte limits, and per-directory traversal limits already prevent
  unbounded resource consumption regardless of the planner's decisions.
- **Future integration path**: A later milestone can make the executor
  task-construction model directly driven from planned per-unit decisions when
  the execution model supports per-unit cancellation and budget reallocation.

The executor does respect the probe-level `systemd_detected` flag: when systemd
is absent, the `service` task is skipped and the `scheduler` task excludes
systemd timer collection. Other source-absence conditions are handled inside
individual collectors through bounded file existence checks.
