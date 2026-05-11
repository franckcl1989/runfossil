# Architectural Decision Record

This record captures project-level architectural decisions that should remain
stable unless a future design review explicitly changes them.

## Decision 1: Raw Evidence First

Decision:

`runfossil` preserves raw runtime evidence before diagnosis.

Rationale:

- Incident recovery often destroys volatile state.
- Raw data is more durable than early interpretation.
- Offline analysis can improve over time without changing capture behavior.

Consequence:

- The capture path prioritizes raw files, directory listings, symlink targets,
  event windows, metadata records, and native protocol dumps.
- Parsed summaries may exist, but they do not replace raw evidence.

## Decision 2: Root-Only Capture

Decision:

`runfossil capture` is a root-only operation.

Rationale:

- Linux runtime evidence is frequently permission-restricted.
- Non-root captures create ambiguous partial results.
- Incident operators normally have elevated privileges for recovery.

Consequence:

- The CLI must fail early when effective UID is not zero.
- Permission failures can still occur and must be recorded in the manifest.

## Decision 3: Project-Owned Code Forbids Unsafe

Decision:

Every project-owned Rust crate must use:

```rust
#![forbid(unsafe_code)]
```

Rationale:

- The project runs as root on production hosts.
- Safe Rust reduces implementation risk.
- A hard policy prevents unsafe code from entering as an optimization shortcut.

Consequence:

- Low-level Linux functionality must use safe abstractions.
- Features without a safe design may be delayed or marked unsupported.
- Third-party dependencies with internal unsafe code require explicit review.

## Decision 4: No External Command Collection

Decision:

Collectors must not invoke external commands.

Rationale:

- Command output is unstable across distributions and versions.
- Command startup and behavior are harder to budget.
- Commands add hidden dependencies and side-effect risk.
- Native collection is easier to test and reason about.

Consequence:

- Collection must use Rust-native access to Linux interfaces.
- Tools such as `ip`, `ss`, `lsof`, `systemctl`, `journalctl`, `docker`,
  `smartctl`, `nvme`, `ipmitool`, and `nvidia-smi` are not used.
- Some hardware or vendor-specific features may remain unsupported until native
  Rust collection is designed.

## Decision 5: Directory-First Snapshot

Decision:

Capture writes a snapshot directory first. Packaging and compression are
post-capture steps.

Rationale:

- A directory is inspectable during and after capture.
- Partial evidence survives interruption.
- Compression should not delay incident recovery.

Consequence:

- `CAPTURE_COMPLETE` is the completion marker.
- Missing completion means partial snapshot, not worthless snapshot.
- `pack` must preserve raw evidence exactly.

## Decision 6: Dynamic Planner Instead of User Capture Configuration

Decision:

The normal capture path does not expose a large configuration surface. The
planner derives the best capture plan from host state.

Rationale:

- Incident-time configuration slows operators down.
- The best plan depends on pressure, scale, kernel features, and available
  runtime sources.
- Defaults should encode project expertise.

Consequence:

- Planner decisions must be recorded in `plan.json`.
- The project should invest in host probes and deterministic heuristics.
- Future operator controls should be small and goal-oriented, not a large source
  checklist.

## Decision 7: Modular Workspace

Decision:

The implementation will use a Rust workspace with separate crates for CLI, core
models, planning, storage, filesystem helpers, source-specific collectors, and
packaging.

Rationale:

- Source families evolve independently.
- The core evidence model should stay stable.
- Collector crates should not own snapshot layout.

Consequence:

- Dependency direction must remain acyclic.
- New source families should be added through collector crates and planner
  extensions.

## Decision 8: Rust 1.95 Baseline

Decision:

The project targets Rust 1.95 as the baseline toolchain.

Rationale:

- The project can use current stable language features.
- A clear baseline simplifies CI and dependency policy.

Consequence:

- Newer stable features may be used when they reduce complexity.
- The project should avoid unnecessary dependencies when Rust 1.95 provides a
  standard feature.

## Decision 9: Snapshot Specification Is a Contract

Decision:

The snapshot specification is a project contract, not an incidental
implementation detail.

Rationale:

- Offline analysis depends on stable evidence layout.
- Operators and engineers need to understand partial snapshots.
- Future tools should be able to inspect old captures.

Consequence:

- Changes to format semantics require documentation updates.
- Manifest status vocabulary should be extended carefully.
- Backward compatibility matters once implementation begins.
