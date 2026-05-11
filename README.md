# runfossil

`runfossil` is a pure Rust Linux runtime snapshot tool for incident forensics.
It captures raw system, process, network, storage, cgroup, service, security,
and container runtime state before a reboot, process restart, or other recovery
action destroys the original evidence.

The project is intentionally raw-first: the capture path preserves runtime
evidence for later offline analysis instead of trying to diagnose the incident
on the production host.

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
- [Architectural Decision Record](docs/ARCHITECTURAL_DECISION_RECORD.md)
- [Implementation Plan](docs/IMPLEMENTATION_PLAN.md)

## Current Status

The repository is in the design phase. The current work defines the behavioral
contracts, output format, collection boundaries, and implementation constraints
before the Rust workspace and crates are introduced.
