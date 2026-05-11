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

- The capture command must fail early when effective UID is not zero.
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

## Decision 10: Snapshot Control Files Use Explicit Schemas

Decision:

`manifest.json`, `plan.json`, and `errors.jsonl` must have explicit required
fields, path rules, timestamp rules, and schema compatibility behavior.

Rationale:

- Offline analysis should not depend on incidental implementation details.
- Partial snapshots need predictable interpretation even when capture is
  interrupted.
- Future versions should be able to add metadata without breaking old readers.

Consequence:

- Schema changes require updates to the Snapshot Specification.
- Readers should ignore unknown fields in known objects.
- Raw evidence files are not rewritten for schema migration.

## Decision 11: Intent, Planning, and Outcomes Use Separate Vocabularies

Decision:

Coverage decisions, plan decisions, and manifest statuses are separate
vocabularies with an explicit mapping.

Rationale:

- Static scope decisions are not the same thing as per-host scheduling decisions.
- A limited plan can still produce a successful capture.
- A deferred native source should not be confused with a missing source or failed
  read.

Consequence:

- The Coverage Decision Matrix owns static intent.
- `plan.json` owns per-capture scheduling decisions.
- `manifest.json` owns final object outcomes.
- New vocabulary values require updates to the Snapshot Specification, Capture
  Planning Strategy, Coverage Decision Matrix, Glossary, and Requirements
  Traceability Matrix.

## Decision 12: AI Tooling Uses Repository-Owned Instructions

Decision:

AI-assisted development must start from repository-owned instructions. `AGENTS.md`
is the canonical AI agent entry point, and tool-specific files defer to it.

Rationale:

- The project has strict safety, scope, and snapshot compatibility constraints.
- Different AI tools should not develop separate interpretations of the design.
- Context routing reduces hallucinated behavior and unnecessary document loading.

Consequence:

- AI workflow changes must update `AGENTS.md` or the AI documents under `docs/`.
- Tool-specific files should stay thin unless a tool requires a special format.
- AI skill specs and prompt templates must preserve the same project constraints
  as the design baseline.

## Decision 13: Documentation-First Implementation Gate

Decision:

Code implementation must not begin until the non-code project documentation
passes the Documentation Readiness Gate and Design Review Checklist.

Rationale:

- `runfossil` will run privileged capture paths on production Linux hosts.
- Snapshot format, safety policy, source coverage, and vocabulary contracts need
  to be stable before crates encode them.
- Fixing ambiguity in documentation is cheaper and safer than discovering it
  after implementation has started.

Consequence:

- Milestone 0 owns documentation readiness.
- Milestone 1 cannot start while either readiness gate has a blocker.
- Implementation findings that expose contract gaps must update documentation
  before the behavior is treated as accepted.

## Decision 14: Repository-Local AI Work Boundary

Decision:

AI-assisted work must stay inside this repository unless the user explicitly
asks for project-external work.

Rationale:

- The design baseline and validation gates are repository-owned.
- Project-external edits can bypass the documented safety, traceability, and
  review controls.
- Keeping work local protects unrelated files and makes review scope clear.

Consequence:

- AI agent instructions must state the repository-local work boundary.
- Documentation readiness review includes repository-local scope checks.
- Any future project-external workflow requires an explicit user request and
  separate review of its safety and ownership impact.

## Decision 15: AI-Driven Development Approach

Decision:

The project is designed for AI-driven development. All design documents, AI
instruction files, skill specs, and prompt templates are repository-owned and
must be sufficient for an AI coding agent to contribute correctly without
project-external context.

Rationale:

- `runfossil` will run privileged capture paths on production Linux hosts.
  Human-only review cannot guarantee every constraint is preserved if AI tools
  lack clear, structured guidance.
- Repository-owned AI instructions reduce the risk of different AI tools
  developing conflicting interpretations of the design.
- The documentation-first gate ensures AI tools have stable contracts before
  generating implementation code.

Consequence:

- `AGENTS.md` is the canonical AI entry point.
- All tool-specific AI files defer to `AGENTS.md`.
- AI context bundles, skill specs, and prompt templates are maintained as
  first-class project artifacts.
- AI tool configuration directories (`.codex/`, `.agents/`) are reserved for
  tool-specific content that does not contradict project policy.

## Decision 16: AI Tool Configuration Directories

Decision:

The repository reserves `.codex/` for Codex-specific configuration and
`.agents/` for agent-specific configuration. These directories are initially
empty by design and must only contain content compatible with the project
constraints defined in `AGENTS.md`.

Rationale:

- Version-controlled AI tool configuration ensures consistent behavior across
  contributors and sessions.
- Tool-specific configuration is separated from tool-agnostic project policy.
- Empty directories signal that configuration is pending while preventing
  undocumented tool behavior from being committed.

Consequence:

- Content in `.codex/` must not contradict `AGENTS.md` or project safety policy.
- Content in `.agents/` must remain compatible with the design baseline.
- AI tooling changes that affect repository behavior should update these
  directories and the relevant AI documentation files.

## Decision 17: AI-Generated Code Is Subject to All Project Constraints

Decision:

Code generated by AI tools must satisfy every constraint that applies to
human-written code: `#![forbid(unsafe_code)]`, no external-command collection,
bounded execution, root-only capture, and snapshot contract compatibility.

Rationale:

- The project's safety and forensic value depend on the code's behavior, not
  its origin.
- A lower standard for AI-generated code would create an unmanageable gap
  between policy and practice.
- Compile-time enforcement (workspace lints, clippy.toml disallowed methods)
  catches many violations automatically.

Consequence:

- All project constraints are enforced at the workspace and CI level regardless
  of authorship.
- AI-specific risks (RK-031 through RK-036) are controlled by the same
  mechanisms that control human-introduced risks.
- AI skill specs and prompt templates must reference project constraints in
  their instructions.
