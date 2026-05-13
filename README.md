# runfossil

`runfossil` is a pure Rust Linux runtime snapshot tool for incident forensics.
It captures raw system, process, network, storage, cgroup, service, security,
and container-related host state before a reboot, process restart, or other
recovery action destroys the original evidence.

The capture objective is a complete, comprehensive, and effective Linux
operating-system runtime snapshot within the configured collection budget. In
practice, that means all in-scope runtime evidence is captured or explicitly
accounted for as not present, not found, skipped, limited, timed out, failed, or
truncated; the capture path must not exceed the configured time, byte, file,
depth, or concurrency limits to chase additional evidence.

The project is intentionally raw-first: the capture path preserves runtime
evidence for later offline analysis instead of trying to diagnose the incident
on the production host.

`runfossil` is designed for AI-driven development. All design documents, AI
workflow instructions, skill specifications, and prompt templates are
repository-owned and maintained as first-class project artifacts. AI coding
agents should start from [AGENTS.md](AGENTS.md) for all repository work.

## Design Constraints

- Rust 1.95 baseline.
- Workspace architecture with modular collector crates.
- Project-owned Rust code must use `#![forbid(unsafe_code)]`.
- Root-only capture execution model.
- No external command execution for collection.
- Directory-first snapshot output, with packaging and compression as a separate
  post-capture step.
- No user-facing capture configuration in the normal path; the tool derives the
  capture plan from current host state.

## Documentation

- [Design Constraints](#design-constraints)
- [License](LICENSE)
- [AI Agent Instructions](AGENTS.md)
- [Claude Instructions](CLAUDE.md)
- [Copilot Instructions](.github/copilot-instructions.md)
- [Project Design](docs/PROJECT_DESIGN.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Snapshot Specification](docs/SNAPSHOT_SPECIFICATION.md)
- [Capture Planning Strategy](docs/CAPTURE_PLANNING_STRATEGY.md)
- [Source Taxonomy](docs/SOURCE_TAXONOMY.md)
- [Coverage Decision Matrix](docs/COVERAGE_DECISION_MATRIX.md)
- [Safety and Operations](docs/SAFETY_AND_OPERATIONS.md)
- [Requirements Traceability](docs/REQUIREMENTS_TRACEABILITY.md)
- [Glossary](docs/GLOSSARY.md)
- [Risk Register](docs/RISK_REGISTER.md)
- [Design Review Checklist](docs/DESIGN_REVIEW_CHECKLIST.md)
- [Documentation Readiness Gate](docs/DOCUMENTATION_READINESS_GATE.md)
- [Documentation Readiness Review](docs/DOCUMENTATION_READINESS_REVIEW.md)
- [Architectural Decision Record](docs/ARCHITECTURAL_DECISION_RECORD.md)
- [Implementation Plan](docs/IMPLEMENTATION_PLAN.md)
- [Release Readiness](docs/RELEASE_READINESS.md)
- [AI Context Index](docs/AI_CONTEXT_INDEX.md)
- [AI Development Guide](docs/AI_DEVELOPMENT_GUIDE.md)
- [AI Skill Specs](docs/AI_SKILL_SPECS.md)
- [AI Prompt Library](docs/AI_PROMPT_LIBRARY.md)

## Current Status

The repository has moved from the documentation-only design phase through the
baseline implementation milestones. Milestones 1 through 8 have working Rust
implementations for the workspace, snapshot store, planner, core collectors,
packaging, and inspection. Milestone 9 hardening and release-readiness work is
in progress; automated gates pass, and release validation still depends on the
remaining external privileged test matrix and signing workflow.

The design baseline defines behavioral contracts, output format, collection
boundaries, vocabulary mappings, safety policy, and implementation constraints
that the code must preserve.

Milestone 0 passed the [Documentation Readiness Gate](docs/DOCUMENTATION_READINESS_GATE.md)
and the [Design Review Checklist](docs/DESIGN_REVIEW_CHECKLIST.md) before code
was introduced.

The historical pre-implementation readiness result is recorded in the
[Documentation Readiness Review](docs/DOCUMENTATION_READINESS_REVIEW.md). It
does not replace the Milestone 9 release-readiness validation, which is tracked
separately in [Release Readiness](docs/RELEASE_READINESS.md).

AI tools should start from [AGENTS.md](AGENTS.md) and use the
[AI Context Index](docs/AI_CONTEXT_INDEX.md) to choose task-specific design
documents.
