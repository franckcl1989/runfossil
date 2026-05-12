# AI Skill Specs

## Purpose

This document defines project-specific AI skills that should guide future Codex
or agent workflows. These are skill specifications, not installed global skills.
If a tool supports reusable skills, create installed skills from these specs and
keep this document as the project-owned source of truth.

## Skill Design Rules

- Keep each skill narrow.
- Load only the documents needed for the skill.
- Make the expected output explicit.
- Require validation steps.
- Do not let skills override `AGENTS.md` or project safety policy.
- Keep work inside this repository unless the user explicitly asks otherwise.

## runfossil-design-steward

Use for product scope, architecture, requirement, or ADR changes.

Read:

- `AGENTS.md`
- `docs/PROJECT_DESIGN.md`
- `docs/ARCHITECTURE.md`
- `docs/ARCHITECTURAL_DECISION_RECORD.md`
- `docs/REQUIREMENTS_TRACEABILITY.md`
- `docs/RISK_REGISTER.md`

Output:

- affected canonical documents;
- required dependent-document updates;
- any changed invariants;
- validation performed.

Validation:

- check that any new constraint appears in Requirements Traceability;
- check that any architectural decision change appears in ADR;
- run `git diff --check`.

## runfossil-snapshot-contract-steward

Use for snapshot layout, manifest fields, plan fields, errors, statuses,
completion semantics, path rules, schema evolution, or archive semantics.

Read:

- `docs/SNAPSHOT_SPECIFICATION.md`
- `docs/GLOSSARY.md`
- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/REQUIREMENTS_TRACEABILITY.md`
- `docs/ARCHITECTURAL_DECISION_RECORD.md`

Output:

- schema or vocabulary change summary;
- compatibility impact;
- reader/writer expectations;
- dependent documents updated.

Validation:

- every new field has type, requiredness, and meaning;
- every new status or decision has a vocabulary entry and mapping;
- partial snapshot semantics remain unambiguous.

## runfossil-coverage-matrix-maintainer

Use for adding, removing, or changing source families, domains, coverage units,
priorities, modes, deferred-native decisions, or exclusions.

Read:

- `docs/SOURCE_TAXONOMY.md`
- `docs/COVERAGE_DECISION_MATRIX.md`
- `docs/CAPTURE_PLANNING_STRATEGY.md`
- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/REQUIREMENTS_TRACEABILITY.md`

Output:

- source ownership decision;
- coverage row changes;
- planner impact;
- safety and risk impact.

Validation:

- every L1 family has a coverage section;
- every L2 domain maps to at least one row;
- every row has decision, priority, mode, and rationale;
- no deferred-native source is replaced by command output.

## runfossil-planner-steward

Use for planner phases, priorities, budgets, pressure handling, skip policy,
dynamic signals, or deterministic planning.

Read:

- `docs/CAPTURE_PLANNING_STRATEGY.md`
- `docs/SNAPSHOT_SPECIFICATION.md`
- `docs/COVERAGE_DECISION_MATRIX.md`
- `docs/SAFETY_AND_OPERATIONS.md`

Output:

- changed planner rule;
- affected plan decision values;
- expected manifest status effects;
- test fixture expectations for future implementation.

Validation:

- `plan.json` remains explainable offline;
- high pressure reduces deep work rather than disabling capture;
- `limited`, `skipped_by_policy`, `unsupported`, and `not_present` remain distinct.

## runfossil-safety-reviewer

Use for privilege, non-destructive behavior, sensitive data, dependencies,
external command policy, output safety, or performance risk.

Read:

- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/RISK_REGISTER.md`
- `docs/PROJECT_DESIGN.md`
- `docs/COVERAGE_DECISION_MATRIX.md`

Output:

- safety finding list ordered by severity;
- affected risks;
- required mitigations;
- whether ADR or requirements must change.

Validation:

- no runtime mutation is introduced;
- no external command collection is introduced;
- no unbounded source is made required;
- sensitive raw evidence remains protected by policy.

## runfossil-doc-consistency-auditor

Use for broad documentation consistency checks before implementation milestones,
before releases, or after large design edits.

Read:

- `AGENTS.md`
- `docs/AI_CONTEXT_INDEX.md`
- `docs/DOCUMENTATION_READINESS_GATE.md`
- `docs/DESIGN_REVIEW_CHECKLIST.md`
- `docs/REQUIREMENTS_TRACEABILITY.md`
- `docs/RISK_REGISTER.md`
- `docs/GLOSSARY.md`

Output:

- blocking inconsistencies;
- non-blocking cleanup suggestions;
- stale or undefined terms;
- validation commands and results.

Validation:

- check the Documentation Readiness Gate before implementation work;
- check that AI guidance preserves repository-local work boundaries;
- run `git diff --check`;
- check relative Markdown links;
- search for placeholder markers and stale vocabulary.

## runfossil-code-reviewer

Use for reviewing Rust code (AI-generated or human-written) against project
constraints.

Read:

- `AGENTS.md`
- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/ARCHITECTURE.md`
- `docs/SNAPSHOT_SPECIFICATION.md`
- `docs/GLOSSARY.md`
- task-specific design docs if the code touches a specific domain.

Output:

- violations of `#![forbid(unsafe_code)]`;
- external command invocations;
- unbounded reads, traversals, or concurrency;
- vocabulary divergence from `runfossil-core` types;
- missing tests for behavioral risk;
- snapshot contract violations.

Validation:

- run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo check --workspace --all-targets`, and `cargo test --workspace`;
- verify no unsafe code in project crates;
- verify `clippy.toml` disallowed methods still reject `Command`;

## runfossil-ai-config-maintainer

Use for creating, editing, or reviewing files in `.codex/` and `.agents/`
directories.

Read:

- `AGENTS.md`
- `docs/AI_CONTEXT_INDEX.md`
- `docs/AI_DEVELOPMENT_GUIDE.md`
- `docs/AI_SKILL_SPECS.md`
- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/GLOSSARY.md`

Output:

- changed configuration files;
- compatibility impact on existing AI workflows;
- required documentation updates.

Validation:

- configuration does not contradict `AGENTS.md`;
- configuration does not weaken safety policy;
- configuration remains tool-specific and does not duplicate canonical policy;
- `docs/AI_CONTEXT_INDEX.md` and `docs/AI_DEVELOPMENT_GUIDE.md` references are
  current.

## runfossil-rust-implementer

Use only after the user explicitly asks to begin implementation.

Read:

- `AGENTS.md`
- `docs/DOCUMENTATION_READINESS_GATE.md`
- `docs/DESIGN_REVIEW_CHECKLIST.md`
- `docs/IMPLEMENTATION_PLAN.md`
- `docs/ARCHITECTURE.md`
- task-specific design docs from `AI_CONTEXT_INDEX.md`

Output:

- files changed;
- milestone targeted;
- tests added or run;
- design docs updated if implementation changed a contract.

Validation:

- Documentation Readiness Gate has no blocker before implementation starts;
- Rust 1.95-compatible commands once the workspace exists;
- `cargo fmt`;
- `cargo test`;
- no project-owned unsafe code;
- no external command collection.
