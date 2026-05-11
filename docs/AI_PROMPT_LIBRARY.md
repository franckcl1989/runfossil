# AI Prompt Library

## Purpose

These prompts are reusable starting points for AI tools working on `runfossil`.
They are intentionally explicit about context, constraints, and validation.

Replace bracketed text before use.

## Design Consistency Review

```text
You are reviewing the runfossil documentation design baseline.

Read AGENTS.md and docs/AI_CONTEXT_INDEX.md first. Then use the Design
Stewardship and Coverage and Planner context bundles.

Task:
Check whether [changed topic or files] is complete, logically self-consistent,
and semantically aligned with the project constraints.

Prioritize:
- contradictions between canonical documents;
- undefined or stale terms;
- missing requirement, risk, ADR, or checklist updates;
- safety policy drift;
- snapshot contract drift.

Return findings first, ordered by severity, with file and line references.
Then list validation commands run.
```

## Snapshot Contract Change

```text
You are updating the runfossil snapshot contract.

Read AGENTS.md, docs/SNAPSHOT_SPECIFICATION.md, docs/GLOSSARY.md,
docs/REQUIREMENTS_TRACEABILITY.md, docs/RISK_REGISTER.md, and
docs/ARCHITECTURAL_DECISION_RECORD.md.

Task:
Change [snapshot field/status/path/schema behavior].

Rules:
- Every machine-readable field must have type, requiredness, and meaning.
- Every new status or plan decision must be added to the vocabulary and mapping.
- Partial snapshot semantics must remain unambiguous.
- Raw evidence must not be rewritten for schema migration.

After editing, run git diff --check and check relative Markdown links if links
changed.
```

## Coverage Matrix Change

```text
You are updating runfossil source coverage.

Read AGENTS.md, docs/SOURCE_TAXONOMY.md, docs/COVERAGE_DECISION_MATRIX.md,
docs/CAPTURE_PLANNING_STRATEGY.md, docs/SAFETY_AND_OPERATIONS.md, and
docs/REQUIREMENTS_TRACEABILITY.md.

Task:
Add or change coverage for [source/domain/object].

Rules:
- Assign L1 source ownership and L2 domain.
- Add exactly one primary coverage unit unless explicitly excluding the source.
- Include decision, priority, mode, and rationale.
- Do not replace deferred-native sources with command output.
- Update requirements and risks if scope or safety changes.

Report changed rows and validation run.
```

## Future Implementation Task

```text
You are implementing runfossil code. Only proceed because the user explicitly
requested implementation.

Read AGENTS.md, docs/DOCUMENTATION_READINESS_GATE.md,
docs/DESIGN_REVIEW_CHECKLIST.md, docs/IMPLEMENTATION_PLAN.md,
docs/ARCHITECTURE.md, and the task-specific context bundle from
docs/AI_CONTEXT_INDEX.md.

Task:
Implement [milestone or feature].

Rules:
- Do not begin if the Documentation Readiness Gate or Design Review Checklist has
  a blocker.
- Keep work inside this repository unless the user explicitly asks otherwise.
- Preserve Rust 1.95 compatibility.
- Add #![forbid(unsafe_code)] to project-owned crates.
- Do not invoke external commands for collection.
- Keep collectors separate from snapshot layout ownership.
- Add focused tests or fixtures for the behavior changed.
- Update docs if implementation reveals a contract gap.

Run the repository's format, lint, build, and test commands if they exist.
```

## Documentation Readiness Review

```text
You are reviewing whether runfossil is ready to begin code implementation.

Read AGENTS.md, docs/DOCUMENTATION_READINESS_GATE.md,
docs/DESIGN_REVIEW_CHECKLIST.md, docs/REQUIREMENTS_TRACEABILITY.md,
docs/RISK_REGISTER.md, docs/ARCHITECTURAL_DECISION_RECORD.md, and
docs/GLOSSARY.md.

Task:
Check whether the non-code documentation baseline has any blocker before
implementation.

Rules:
- Treat unresolved documentation gaps as blockers for implementation.
- Do not create code, tests, fixtures, or generated files.
- Keep the review inside this repository unless the user explicitly asks
  otherwise.
- Verify README links, canonical ownership, traceability, risk coverage,
  vocabulary consistency, and placeholder markers.
- Do not claim any feature is implemented when it exists only in documentation.

Run git diff --check and report any link or vocabulary checks performed.
```

## Safety Review

```text
You are doing a safety review for runfossil.

Read AGENTS.md, docs/SAFETY_AND_OPERATIONS.md, docs/RISK_REGISTER.md,
docs/PROJECT_DESIGN.md, and relevant coverage or implementation files.

Task:
Review [change or design] for production safety.

Find issues involving:
- runtime mutation;
- process control;
- external command collection;
- unsafe project-owned Rust;
- unbounded reads, traversal, concurrency, or output growth;
- sensitive data handling;
- partial snapshot misinterpretation.

Return findings first, ordered by severity, with file and line references.
```

## AI Workflow Update

```text
You are improving runfossil AI workflow support.

Read AGENTS.md, CLAUDE.md, .github/copilot-instructions.md,
docs/AI_CONTEXT_INDEX.md, docs/AI_DEVELOPMENT_GUIDE.md,
docs/AI_SKILL_SPECS.md, and docs/AI_PROMPT_LIBRARY.md.

Task:
Update AI guidance for [tool/workflow/skill].

Rules:
- Keep AGENTS.md canonical.
- Keep tool-specific files thin adapters unless tool-specific behavior is needed.
- Keep skill specs narrow and trigger-oriented.
- Update README if new AI-facing docs are added.

Run git diff --check and check links if links changed.
```

## Cross-Document Consistency Audit

```text
You are auditing runfossil documentation for cross-document consistency.

Read AGENTS.md, docs/AI_CONTEXT_INDEX.md, docs/DOCUMENTATION_READINESS_GATE.md,
docs/DESIGN_REVIEW_CHECKLIST.md, docs/GLOSSARY.md, and all documents whose
topics intersect with the changed area.

Task:
Verify that [changed documents] do not introduce contradictions, vocabulary
drift, broken links, unupdated dependent documents, or missing canonical
updates.

Rules:
- Every term used as a contract must appear in Glossary.
- Every source family changed must update Source Taxonomy and Coverage Decision
  Matrix.
- Every snapshot field or status changed must update Snapshot Specification.
- Every scope or safety change must update Requirements Traceability and Risk
  Register.
- Vocabulary must remain consistent across all documents that use it.

Report contradictions first, then stale terms, then broken links. Run
git diff --check and link validation.
```

## AI Tool Configuration Change

```text
You are updating runfossil AI tool configuration.

Read AGENTS.md, docs/AI_DEVELOPMENT_GUIDE.md, docs/AI_SKILL_SPECS.md,
docs/AI_CONTEXT_INDEX.md, docs/SAFETY_AND_OPERATIONS.md, and
docs/GLOSSARY.md.

Task:
Add or edit configuration files in [.codex/ or .agents/ directory].

Rules:
- Configuration must not contradict AGENTS.md or project safety policy.
- Configuration must not duplicate canonical project policy.
- Keep tool-specific configuration separate from tool-agnostic documentation.
- Update AI_CONTEXT_INDEX.md, AI_DEVELOPMENT_GUIDE.md, and AI_SKILL_SPECS.md
  when configuration affects AI workflows.

Report files changed and validation performed.
```
