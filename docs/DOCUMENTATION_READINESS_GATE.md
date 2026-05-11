# Documentation Readiness Gate

## Purpose

This gate defines the documentation-first condition for `runfossil`.
Implementation must not begin until the non-code project documents are complete,
consistent, and validated against this gate.

The gate turns "finish the docs before code" into a project rule. It is not a
claim that documentation can be perfect forever; it means the design baseline is
implementation-quality and has no known blocking ambiguity.

The checklist below is a reusable template. Each item is marked `- [ ]` so it
can be re-evaluated when the design baseline changes materially. For the most
recent pass/fail results, see
[DOCUMENTATION_READINESS_REVIEW.md](DOCUMENTATION_READINESS_REVIEW.md).

## Scope

This gate applies to every non-code project document in this repository:

- `README.md`
- `AGENTS.md`
- `CLAUDE.md`
- `.github/copilot-instructions.md`
- all Markdown files under `docs/`

It does not authorize work outside this repository. It also does not authorize a
Rust workspace, crates, generated files, fixtures, or tests. Those remain future
implementation work and require an explicit user request after this gate passes.

## Gate Rule

Code implementation may start only when all of the following are true:

- The Design Review Checklist passes with no unresolved blocker.
- This Documentation Readiness Gate passes with no unresolved blocker.
- The Requirements Traceability covers every explicit project
  requirement.
- The Risk Register records every known design risk that could affect safety,
  forensic value, compatibility, or implementation direction.
- The ADR records every major project constraint and any change to those
  constraints.
- Validation commands for documentation-only work pass.

If any item fails, the next task is a documentation task, not implementation.

## Canonical Ownership

Each design topic must have one canonical owner:

| Topic | Canonical document |
|---|---|
| Product scope and non-goals | Project Design |
| Architecture and component boundaries | Architecture |
| Snapshot layout and schemas | Snapshot Specification |
| Source boundaries | Source Taxonomy |
| Coverage decisions | Coverage Decision Matrix |
| Planner behavior | Capture Planning Strategy |
| Safety and operations | Safety and Operations |
| Implementation order | Implementation Plan |
| Requirements | Requirements Traceability |
| Risks | Risk Register |
| Terms | Glossary |
| Architectural decisions | ADR |
| Review gates | Design Review Checklist and this document |
| AI workflow | AGENTS and AI documents |

Dependent documents may summarize a rule, but they must not contradict the
canonical owner. When a conflict is found, preserve the stricter safety or
compatibility rule while updating the weaker document.

## Readiness Checklist

### Scope and Product Boundary

- [ ] The project is still described as a pure Rust Linux runtime snapshot tool.
- [ ] Project work stays inside this repository unless the user explicitly asks
      otherwise.
- [ ] Implementation files, if present, were introduced only after explicit user
      request and a passing documentation readiness review.
- [ ] Excluded domains remain explicit: static configuration, application data,
      language runtime internals, database internals, Kubernetes control-plane
      state, cloud APIs, package databases, source code, and full historical
      logs.
- [ ] No document claims an implementation exists before code is added.

### Safety and Operations

- [ ] `runfossil capture` is root-only everywhere it is described.
- [ ] Packaging and inspection operate on existing artifacts and do not require
      root by default.
- [ ] Project-owned Rust code is required to use `#![forbid(unsafe_code)]`.
- [ ] Collectors are prohibited from invoking external commands.
- [ ] Collection is raw-first, bounded, non-destructive, and directory-first.
- [ ] Sensitive snapshot output and restrictive permissions are documented.

### Snapshot Contract

- [ ] Snapshot control files have explicit required fields.
- [ ] Schema compatibility and unknown-field behavior are defined.
- [ ] Relative artifact path rules forbid absolute paths and `..`.
- [ ] `CAPTURE_COMPLETE` and partial snapshot semantics are unambiguous.
- [ ] Coverage decisions, plan decisions, and manifest statuses remain separate
      vocabularies with an explicit mapping.

### Source and Planner Coverage

- [ ] Every L1 source family in Source Taxonomy has coverage decisions.
- [ ] Every L2 domain maps to at least one coverage unit.
- [ ] Every L3 object family has exactly one primary coverage unit or an explicit
      exclusion.
- [ ] Every coverage unit has decision, priority, mode, and rationale.
- [ ] Planner phases, priorities, budgets, skip policy, and pressure behavior
      are documented.

### Traceability and Change Control

- [ ] Every explicit requirement maps to canonical documents and acceptance
      criteria.
- [ ] Every major constraint is recorded in ADR.
- [ ] Every known material risk has a mitigation or accepted status.
- [ ] Every project-specific term used as a contract appears in Glossary.
- [ ] Any changed source, status, schema rule, path rule, milestone, or AI
      workflow updates its dependent documents in the same change.

### AI Workflow

- [ ] `AGENTS.md` remains the canonical AI instruction file.
- [ ] Tool-specific AI files defer to `AGENTS.md`.
- [ ] AI Context Index routes common tasks to authoritative documents.
- [ ] AI Development Guide requires documentation/design handling before
      implementation when the task is unclear.
- [ ] AI Skill Specs and Prompt Library do not tell agents to bypass validation,
      invent implemented behavior, or work outside the repository.
- [ ] `.codex/` directory content, if present, is consistent with `AGENTS.md`.
- [ ] `.agents/` directory content, if present, is consistent with `AGENTS.md`.
- [ ] AI-specific risks are recorded in the Risk Register.
- [ ] AI Development Guide covers `.codex/` and `.agents/` directory management.

### Repository Hygiene

- [ ] README links all project documents.
- [ ] Relative Markdown links resolve.
- [ ] Placeholder markers are removed or intentionally tracked in a design
      document.
- [ ] Stale vocabulary is removed when status, decision, source, or schema terms
      change.
- [ ] `git diff --check` passes.

## Validation Commands

For documentation-only readiness work, run:

```bash
git diff --check
```

Also search for unresolved placeholders or stale review markers:

```bash
rg -n "[T]ODO|[T]BD|[F]IXME|[X]XX|[P]LACEHOLDER|[O]pen Design Questions|to be decide[d]|to be define[d]" .
```

When Markdown links change, check all relative Markdown links manually or with a
local link-checking command. Do not use network-dependent validation for this
project gate unless the user explicitly asks for it.

## Passing Result

Passing this gate means the non-code design baseline is ready for code
implementation. It does not mean any feature is implemented.

After implementation begins, this gate remains active: any code change that
discovers a contract gap must update the relevant documentation before treating
the behavior as accepted.
