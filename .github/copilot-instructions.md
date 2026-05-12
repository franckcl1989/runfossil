# Copilot Instructions

`runfossil` is a pure Rust Linux runtime snapshot tool in Milestone 9 hardening
and release-readiness work.

Follow the repository policy in `AGENTS.md`. In particular:

- keep implementation suggestions within the current milestone unless the user
  explicitly asks to change scope;
- do not suggest implementation while the documentation readiness or design
  review gate has a blocker;
- keep suggestions inside this repository unless the user explicitly asks
  otherwise;
- preserve root-only capture, safe project-owned Rust, no external command
  collection, raw-first evidence, bounded execution, and directory-first
  snapshots;
- keep snapshot schema, coverage decisions, plan decisions, and manifest statuses
  aligned with the canonical docs;
- update related documentation whenever a project term, source family, coverage
  decision, status value, or architectural decision changes.

Use `docs/AI_CONTEXT_INDEX.md` to choose the right design documents before
suggesting edits.

Copilot-specific configuration, if needed, should use `.codex/` or `.agents/`
and must not contradict project policy.
