# Design Review Checklist

## Purpose

This checklist is the design review gate before implementation. It is meant to
be run manually whenever the design baseline changes materially.

The checklist protects the project from drifting away from its core constraints:
safe project-owned Rust code, root-only capture, native collection, raw-first
evidence, bounded execution, and directory-first snapshots.

Each item is marked `- [ ]` so the checklist can be re-evaluated when the design
baseline changes. For the most recent pass/fail results, see
[DOCUMENTATION_READINESS_REVIEW.md](DOCUMENTATION_READINESS_REVIEW.md).

## Scope Gate

- [ ] The project remains a Linux runtime raw snapshot tool.
- [ ] Application data, business data, language runtime internals, database
      internals, Kubernetes control-plane state, cloud control-plane APIs, and
      full historical logs remain outside core scope.
- [ ] New source families are placed under the existing L1 taxonomy unless they
      are genuinely independent Linux runtime sources.
- [ ] Source ownership is documented in Source Taxonomy.
- [ ] Collection decisions are documented in Coverage Decision Matrix.

## Safety Gate

- [ ] Project-owned crates are required to use `#![forbid(unsafe_code)]`.
- [ ] Capture remains root-only.
- [ ] Offline artifact commands do not require root unless a future feature has
      a specific documented reason.
- [ ] Collectors do not mutate runtime state.
- [ ] Collectors do not signal, pause, ptrace, or control target processes.
- [ ] Device nodes are not read as arbitrary streams.
- [ ] DebugFS, TraceFS, BPF, and policy-sensitive sources are bounded or
      deferred.
- [ ] Snapshot output is treated as sensitive.

## Native Collection Gate

- [ ] No collector decision depends on external command execution.
- [ ] Sources that require command-line tools today are marked `deferred-native`.
- [ ] Native netlink, native socket, native protocol, and bounded filesystem
      collection modes are preferred.
- [ ] Vendor or hardware-management features without safe native designs remain
      deferred.

## Snapshot Artifact Gate

- [ ] Snapshot output is directory-first.
- [ ] Packaging and compression remain post-capture operations.
- [ ] `CAPTURE_COMPLETE` semantics are preserved.
- [ ] Partial snapshots remain valid evidence.
- [ ] `manifest.json` records object outcomes.
- [ ] `plan.json` records planner decisions.
- [ ] `errors.jsonl` records runtime errors without requiring whole-capture
      failure.
- [ ] Snapshot status vocabulary remains consistent across documents.
- [ ] Snapshot control files define required fields and compatibility behavior.
- [ ] Relative artifact path rules prevent absolute paths, `..`, and reserved-name
      collisions.

## Planner Gate

- [ ] The planner derives decisions from host state instead of normal user
      configuration.
- [ ] Planner phases remain probe, score, budget, execute.
- [ ] Task priorities P0 through P4 remain meaningful.
- [ ] Every task has timeout, byte, file, depth, and concurrency limits.
- [ ] Planner decisions can be explained offline from `plan.json`.
- [ ] High pressure reduces deep work instead of disabling capture.
- [ ] Incident signals deepen relevant source families only.
- [ ] Plan decisions use the shared vocabulary: `scheduled`, `limited`,
      `skipped_by_policy`, `unsupported`, and `not_present`.

## Coverage Gate

- [ ] Every L1 source family has coverage decisions.
- [ ] Every L2 domain from the source taxonomy is represented by at least one
      coverage unit.
- [ ] Every L3 object family has exactly one primary coverage unit or an explicit
      exclusion.
- [ ] Every coverage unit has decision, priority, mode, and rationale.
- [ ] Non-collected sources have an explicit reason.
- [ ] Large payload sources are limited or metadata-only by default.
- [ ] `deferred-native` sources are not replaced by command output.
- [ ] Explicit exclusions remain documented.
- [ ] Coverage decisions, plan decisions, and manifest statuses remain mapped.

## Architecture Gate

- [ ] CLI, core, planner, store, filesystem helpers, collectors, and packager
      responsibilities remain separate.
- [ ] Collectors do not own snapshot layout.
- [ ] Store owns persistence and artifact format.
- [ ] Planner owns task selection and budgets.
- [ ] Dependency direction remains acyclic.
- [ ] Adding a collector does not require changing unrelated source families.

## Requirements Gate

- [ ] Requirements Traceability maps each explicit requirement to
      canonical documents.
- [ ] Any changed requirement updates the matrix.
- [ ] Any new risk updates Risk Register.
- [ ] Any new project term updates Glossary.
- [ ] Any architectural decision change updates ADR.

## AI Tooling Gate

- [ ] `AGENTS.md` remains the canonical AI agent instruction file.
- [ ] Tool-specific AI files defer to `AGENTS.md` unless they need tool-specific
      behavior.
- [ ] AI Context Index routes each common task type to authoritative documents.
- [ ] AI Skill Specs remain narrow, trigger-oriented, and consistent with project
      safety policy.
- [ ] AI Prompt Library does not tell agents to bypass validation or invent
      implemented behavior.
- [ ] AI instructions keep work inside this repository unless the user explicitly
      asks otherwise.
- [ ] `.codex/` configuration, if present, is compatible with `AGENTS.md` and
      safety policy.
- [ ] `.agents/` configuration, if present, is compatible with `AGENTS.md` and
      safety policy.
- [ ] AI-specific risks (RK-028 through RK-036) are documented and controlled in
      the Risk Register.
- [ ] AI Development Guide includes guidance on `.codex/` and `.agents/`
      directory management.
- [ ] AI Skill Specs include skills for code review, documentation consistency
      audit, and AI tool configuration management.

## Pre-Implementation Exit Criteria

Implementation should not start until:

- [ ] Documentation links from README are current.
- [ ] Documentation Readiness Gate passes with no blocker.
- [ ] Source Taxonomy, Coverage Decision Matrix, Snapshot Specification, Capture
      Planning Strategy, Safety and Operations, ADR, and Implementation Plan are
      mutually consistent.
- [ ] No document depends on an external command workaround.
- [ ] No document leaves source coverage decisions implicit.
- [ ] No document describes root-only behavior ambiguously.
- [ ] No document introduces a new status, decision, or collection mode without
      updating the authoritative vocabulary.
- [ ] AI-assisted development instructions are current and linked from README.
- [ ] Placeholder markers, stale terms, and unresolved open questions are either
      removed or intentionally tracked in Requirements Traceability, Risk
      Register, ADR, or Implementation Plan.
- [ ] `git diff --check` passes for documentation changes.
