# AI Agent Instructions

This file is the canonical operating guide for AI coding agents working on
`runfossil`. Other AI tool entry points should defer to this file instead of
duplicating policy.

## Project State

The repository has entered Milestone 1 implementation. It contains a Rust 1.95
workspace skeleton plus the documentation design baseline. Keep implementation
work scoped to the current user request and the milestone boundaries in
`docs/IMPLEMENTATION_PLAN.md`.

Milestone 0 passed `docs/DOCUMENTATION_READINESS_GATE.md` and
`docs/DESIGN_REVIEW_CHECKLIST.md` before code was introduced. If later
implementation reveals a contract gap, update the relevant documentation before
treating the behavior as accepted.

## Read First

For most tasks, read these files before making changes:

- `README.md`
- `docs/PROJECT_DESIGN.md`
- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/SNAPSHOT_SPECIFICATION.md`
- `docs/COVERAGE_DECISION_MATRIX.md`
- `docs/CAPTURE_PLANNING_STRATEGY.md`
- `docs/GLOSSARY.md`
- `docs/AI_CONTEXT_INDEX.md`

Use `docs/AI_CONTEXT_INDEX.md` to narrow additional reading by task type.

When unclear about where a topic is defined, consult the canonical ownership
table in `docs/DOCUMENTATION_READINESS_GATE.md`.

## Non-Negotiable Constraints

- `runfossil capture` is root-only.
- Packaging and inspection operate on existing artifacts and should not require
  root by default.
- Project-owned Rust code must use `#![forbid(unsafe_code)]`.
- Collectors must not invoke external commands.
- Capture is raw-first, bounded, non-destructive, and directory-first.
- Snapshot control files and vocabularies are contracts.
- Coverage decisions, plan decisions, and manifest statuses are separate
  vocabularies with an explicit mapping.
- Static configuration, application data, Kubernetes control-plane state, cloud
  APIs, language runtime internals, database internals, and full historical logs
  remain outside core scope.
- Documentation readiness is a precondition for code implementation.

## Change Rules

- Keep changes scoped to the requested task.
- Keep work inside this repository unless the user explicitly asks otherwise.
- Preserve user changes already present in the working tree.
- Do not invent implementation behavior that contradicts the design baseline.
- When adding or changing a source family, update Source Taxonomy, Coverage
  Decision Matrix, Capture Planning Strategy, Requirements Traceability, Risk
  Register, and Design Review Checklist as needed.
- When adding or changing a snapshot field, status, schema rule, or path rule,
  update Snapshot Specification, Glossary, Requirements Traceability, Risk
  Register, and ADR as needed.
- When changing a major project constraint, update ADR before treating the change
  as accepted.
- When changing AI workflow instructions, update this file and the relevant AI
  docs under `docs/`.

## Validation

For documentation-only changes:

- Run `git diff --check`.
- Check Markdown links when files or links change.
- Search for stale terms, placeholder markers, and conflicting vocabulary.

For code changes:

- Run the repository's formatting, lint, build, and test commands:
  `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo check`, `cargo test`.
- Verify no project-owned unsafe code is introduced.
- Verify collectors do not execute external commands.
- Verify vocabulary spellings in `runfossil-core` match the snapshot contract.

## AI Tool Configuration

The repository reserves two directories for AI tool configuration:

- `.codex/` for Codex-specific skills, workflows, and settings.
- `.agents/` for agent-specific configuration files.

Content in these directories must not contradict `AGENTS.md`, project safety
policy, or the design baseline. Configuration files should be version-controlled
and reviewed like any other project artifact.

If adding or editing files in `.codex/` or `.agents/`, update
`docs/AI_CONTEXT_INDEX.md`, `docs/AI_DEVELOPMENT_GUIDE.md`, and
`docs/AI_SKILL_SPECS.md` as needed to keep AI workflow documentation current.

## Communication

When reporting results, distinguish between:

- design intent;
- implemented behavior;
- proposed future work;
- validation actually run.

Do not describe a feature as implemented when it exists only in documentation.
