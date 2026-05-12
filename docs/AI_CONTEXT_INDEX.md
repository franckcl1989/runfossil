# AI Context Index

## Purpose

This index helps AI tools load the smallest useful set of project documents for a
task. It protects the design from drift while avoiding unnecessary context.

Always start with `AGENTS.md` when an AI coding agent is doing repository work.

## Core Context

Use this bundle for any non-trivial project change:

- `README.md`
- `docs/PROJECT_DESIGN.md`
- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/GLOSSARY.md`
- `docs/DESIGN_REVIEW_CHECKLIST.md`

## Task Routing

| Task | Read before editing | Usually update |
|---|---|---|
| Change product scope or non-goals | Project Design, Source Taxonomy, Coverage Decision Matrix, Requirements Traceability, ADR | Project Design, Source Taxonomy, Coverage Decision Matrix, Requirements Traceability, Risk Register, ADR |
| Change snapshot layout, schema, fields, statuses, paths, or partial semantics | Snapshot Specification, Glossary, Requirements Traceability, ADR | Snapshot Specification, Glossary, Requirements Traceability, Risk Register, Design Review Checklist, ADR |
| Add or change a source family or domain | Source Taxonomy, Coverage Decision Matrix, Capture Planning Strategy, Safety and Operations | Source Taxonomy, Coverage Decision Matrix, Requirements Traceability, Risk Register, Implementation Plan |
| Add or change a collector delivery milestone | Implementation Plan, Architecture, Coverage Decision Matrix | Implementation Plan, Architecture, Requirements Traceability, Risk Register |
| Change planner behavior, priorities, budgets, or skip policy | Capture Planning Strategy, Snapshot Specification, Coverage Decision Matrix | Capture Planning Strategy, Snapshot Specification, Requirements Traceability, Risk Register |
| Change safety, privilege, dependency, or operational rules | Safety and Operations, Project Design, ADR | Safety and Operations, Risk Register, Requirements Traceability, ADR |
| Review design consistency | Design Review Checklist, Requirements Traceability, Risk Register, Glossary | Design Review Checklist and any document that fails the review |
| Review documentation readiness before implementation | Documentation Readiness Gate, Design Review Checklist, Requirements Traceability, Risk Register, ADR, Glossary | Any document that fails the readiness gate |
| Prepare AI-assisted implementation work | AI Development Guide, Documentation Readiness Gate, Design Review Checklist, AI Skill Specs, AI Prompt Library, AGENTS.md | AI docs, Implementation Plan, Design Review Checklist, Documentation Readiness Gate |
| Manage `.codex/` or `.agents/` configuration | AGENTS.md, AI Development Guide, AI Skill Specs | AGENTS.md, AI Context Index, AI Development Guide, AI Skill Specs |
| Change release hardening, static builds, dependency audit, production validation, or artifact signing | Implementation Plan, Safety and Operations, Risk Register, Release Readiness | Implementation Plan, Risk Register, Release Readiness, README |

## Context Bundles

### Design Stewardship

Use for scope, architecture, and requirement changes:

- `docs/PROJECT_DESIGN.md`
- `docs/ARCHITECTURE.md`
- `docs/ARCHITECTURAL_DECISION_RECORD.md`
- `docs/REQUIREMENTS_TRACEABILITY.md`
- `docs/RISK_REGISTER.md`

### Snapshot Contract

Use for artifact format and offline tooling contracts:

- `docs/SNAPSHOT_SPECIFICATION.md`
- `docs/GLOSSARY.md`
- `docs/SAFETY_AND_OPERATIONS.md`
- `docs/REQUIREMENTS_TRACEABILITY.md`
- `docs/ARCHITECTURAL_DECISION_RECORD.md`

### Coverage and Planner

Use for source coverage and capture selection:

- `docs/SOURCE_TAXONOMY.md`
- `docs/COVERAGE_DECISION_MATRIX.md`
- `docs/CAPTURE_PLANNING_STRATEGY.md`
- `docs/SNAPSHOT_SPECIFICATION.md`
- `docs/SAFETY_AND_OPERATIONS.md`

### AI Tooling

Use for improving prompts, skills, or AI workflows:

- `AGENTS.md`
- `CLAUDE.md`
- `.github/copilot-instructions.md`
- `docs/AI_DEVELOPMENT_GUIDE.md`
- `docs/AI_SKILL_SPECS.md`
- `docs/AI_PROMPT_LIBRARY.md`

### AI Tool Configuration

Use for adding or editing files in `.codex/` or `.agents/`:

- `AGENTS.md`
- `docs/AI_DEVELOPMENT_GUIDE.md`
- `docs/AI_SKILL_SPECS.md`
- `docs/GLOSSARY.md`
- `docs/SAFETY_AND_OPERATIONS.md`

### Pre-Implementation Readiness

Use before creating any Rust workspace, crates, tests, fixtures, or generated
code:

- `docs/DOCUMENTATION_READINESS_GATE.md`
- `docs/DOCUMENTATION_READINESS_REVIEW.md`
- `docs/DESIGN_REVIEW_CHECKLIST.md`
- `docs/REQUIREMENTS_TRACEABILITY.md`
- `docs/RISK_REGISTER.md`
- `docs/ARCHITECTURAL_DECISION_RECORD.md`
- `docs/GLOSSARY.md`

## Context Hygiene

- Do not load every document by default.
- Prefer the narrow task bundle plus direct dependencies.
- When a change introduces a new term, status, decision, or source family, search
  the repository for related vocabulary before editing.
- If two documents conflict, preserve the stricter safety rule and update the
  weaker document.
