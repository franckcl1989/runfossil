# AI Development Guide

## Purpose

This guide defines how AI tools should help develop `runfossil` without eroding
the design baseline. It is tool-neutral and applies to Codex, Claude Code,
Copilot, review agents, and future local automation.

## Operating Model

AI-assisted work should follow this sequence:

1. Classify the task.
2. Load the smallest relevant context bundle from `AI_CONTEXT_INDEX.md`.
3. Identify the authoritative documents for the change.
4. If the task is implementation work, confirm the Documentation Readiness Gate
   and Design Review Checklist have no blockers.
5. Make a narrow edit or implementation.
6. Update dependent documents in the same change.
7. Run the relevant validation commands.
8. Report what changed and what was validated.

## Task Classification

Use these categories:

- Documentation change.
- Design consistency review.
- Snapshot contract change.
- Coverage or planner change.
- Safety or risk change.
- Future implementation change.
- Future test or fixture change.
- AI workflow or skill change.

When the task type is unclear, treat it as a documentation/design task until the
user explicitly requests code.

## Required Guardrails

AI tools must preserve:

- repository-local work unless the user explicitly asks otherwise;
- root-only live capture;
- non-root offline artifact commands;
- project-owned safe Rust;
- no external command collection;
- raw-first evidence;
- directory-first snapshot output;
- bounded and non-destructive collection;
- explicit partial snapshot semantics;
- separate coverage, plan, and manifest vocabularies.

## AI Tool Configuration Directories

The repository reserves two directories for AI tool configuration:

- `.codex/` for Codex-specific skills, workflows, settings, and instructions.
- `.agents/` for agent-specific configuration files.

Rules for these directories:

- Content must not contradict `AGENTS.md`, project safety policy, or the design
  baseline.
- Configuration files should be version-controlled and reviewed like any other
  project artifact.
- Empty directories signal that tool-specific configuration is pending, not
  missing.
- Adding or editing files in `.codex/` or `.agents/` should update
  `AI_CONTEXT_INDEX.md`, this document, and `AI_SKILL_SPECS.md` as needed.

## Configuration Validation

Before committing `.codex/` or `.agents/` content, verify:

- The content does not override project constraints defined in `AGENTS.md`.
- The content does not weaken safety policy, coverage boundaries, or snapshot
  contracts.
- The content remains tool-specific and does not duplicate canonical project
  policy.
- Related AI documentation references are current.

## Documentation Change Workflow

For a documentation change:

1. Read the task-specific context bundle.
2. Update the canonical document first.
3. Update dependent documents only where the change affects their contract.
4. Run `git diff --check`.
5. Check links when adding or editing Markdown links.
6. Search for stale vocabulary when changing terms.

Canonical ownership:

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
| Terminology | Glossary |
| Architectural decisions | ADR |
| Review gates | Design Review Checklist and Documentation Readiness Gate |
| AI workflow | AGENTS and AI documents |

## Future Implementation Workflow

Implementation should not begin until the user explicitly asks for it.

Implementation also should not begin while the Documentation Readiness Gate or
Design Review Checklist has an unresolved blocker. If either gate fails, handle
the documentation/design gap first.

When implementation begins:

1. Start at the milestone named in `IMPLEMENTATION_PLAN.md`.
2. Re-read `docs/DOCUMENTATION_READINESS_GATE.md` and
   `docs/DESIGN_REVIEW_CHECKLIST.md`.
3. Preserve Rust 1.95 compatibility.
4. Add `#![forbid(unsafe_code)]` to every project-owned Rust crate.
5. Keep collectors separate from snapshot layout ownership.
6. Do not shell out to Linux tools for collection.
7. Add tests or fixtures proportional to the behavioral risk.
8. Update design docs when implementation discovers a contract gap.

## AI Review Workflow

AI review should prioritize:

- contradiction between canonical documents;
- vocabulary drift;
- missing dependent-document updates;
- unsafe or destructive collection paths;
- external command workarounds;
- snapshot compatibility risks;
- unbounded traversal, reads, or concurrency;
- claims of implementation that only exist as design.

## Validation Commands

For documentation-only work:

```bash
git diff --check
```

When Markdown links change, also run a local link check or inspect all relative
Markdown links.

For future code work, use the repository's eventual Rust commands. Until those
commands exist, do not claim build or test validation.

## Reporting Standard

AI tools should report:

- files changed;
- contract changes made;
- validation run;
- validation not run and why;
- remaining design or implementation risk.

Avoid saying a feature is implemented when only documentation was changed.
