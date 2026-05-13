# Requirements Traceability

## Purpose

This document maps project requirements to the design documents that define and
verify them. It is the design-phase acceptance matrix for `runfossil`.

The matrix covers explicit user requirements, derived system requirements, and
documentation requirements that protect the design from ambiguity before code
implementation begins.

## Status Vocabulary

```text
accepted
  The requirement is part of the project baseline.

specified
  The requirement is described in canonical design documents.

bounded
  The requirement is accepted with explicit limits or exclusions.

deferred
  The requirement is valid, but implementation depends on a later native design.

excluded
  The requirement is intentionally outside scope.
```

## Traceability Matrix

| ID | Requirement | Status | Canonical documents | Acceptance criteria |
|---|---|---:|---|---|
| R-001 | The project is implemented in Rust. | accepted | Project Design, ADR, Implementation Plan | Workspace code uses Rust as the only implementation language. |
| R-002 | Rust 1.95 is the baseline toolchain. | specified | README, Project Design, ADR, Implementation Plan | Toolchain and CI target Rust 1.95 or newer compatible stable releases. |
| R-003 | Project-owned Rust code must be 100% safe code. | specified | Project Design, Safety and Operations, ADR | Every project crate uses `#![forbid(unsafe_code)]`. |
| R-004 | Third-party dependencies may be used only when justified. | bounded | Project Design, Safety and Operations, ADR | Dependencies are small, reviewable, pinned, and justified when they contain internal unsafe code. |
| R-005 | The project uses a workspace architecture. | specified | Architecture, Implementation Plan | Workspace has separate crates for CLI, core, planning, storage, collectors, and packaging. |
| R-006 | The design must be modular. | specified | Architecture, ADR | Collector families can evolve independently without owning snapshot layout. |
| R-007 | Capture is root-only. | specified | Project Design, Safety and Operations, ADR | `runfossil capture` fails early when effective UID is not zero. |
| R-008 | Packaging and inspection do not require root. | specified | Project Design | `pack` and `inspect` operate on snapshot artifacts, not live privileged sources. |
| R-009 | The tool captures Linux runtime raw snapshot evidence. | specified | Project Design, Source Taxonomy, Coverage Decision Matrix | Runtime source families are mapped to collection decisions. |
| R-010 | The tool preserves raw evidence rather than diagnosing live incidents. | specified | Project Design, ADR, Snapshot Specification | Raw files, listings, symlink targets, metadata, and native responses are primary outputs. |
| R-011 | Snapshot output is directory-first. | specified | Project Design, Snapshot Specification, ADR | Capture produces an inspectable directory before any archive is created. |
| R-012 | Packaging and compression are post-capture steps. | specified | Project Design, Snapshot Specification, Safety and Operations | Compression is never required to preserve incident-time evidence. |
| R-013 | Normal capture has no user-facing configuration. | specified | Project Design, Capture Planning Strategy, ADR | Planner derives capture decisions from host state instead of user-tuned source lists. |
| R-014 | The planner dynamically adapts to host state. | specified | Capture Planning Strategy | Planner records probe, score, budget, and execution decisions. |
| R-015 | The planner must record why work was scheduled, limited, skipped, unsupported, or not present. | specified | Snapshot Specification, Capture Planning Strategy | `plan.json` explains task decisions and skip reasons. |
| R-016 | Collection must not invoke external commands. | specified | Project Design, Architecture, Safety and Operations, ADR | Collectors do not invoke tools such as `ip`, `ss`, `systemctl`, `journalctl`, `docker`, `smartctl`, or `ipmitool`. |
| R-017 | Collection must be Rust-native. | specified | Project Design, Architecture, Coverage Decision Matrix | Native Linux interfaces, netlink, sockets, filesystems, and protocols are used instead of command wrappers. |
| R-018 | Sources requiring native protocol support are not replaced by shell commands. | specified | Coverage Decision Matrix, Safety and Operations, ADR | Sources without accepted native collectors are marked unsupported, deferred, or excluded instead of being command-wrapped. |
| R-019 | `/proc` runtime state is in scope. | specified | Source Taxonomy, Coverage Decision Matrix | `/proc` domains have explicit collection decisions. |
| R-020 | `/sys` runtime state is in scope. | specified | Source Taxonomy, Coverage Decision Matrix | `/sys` domains have explicit collection decisions and traversal limits. |
| R-021 | `/run` runtime state is in scope. | specified | Source Taxonomy, Coverage Decision Matrix | Runtime metadata and bounded trees are captured without blind copying. |
| R-022 | `/dev` runtime device namespace is in scope. | bounded | Source Taxonomy, Coverage Decision Matrix, Safety and Operations | Device nodes are captured as metadata unless explicitly safe as bounded event sources. |
| R-023 | Kernel ring buffer event windows are in scope. | specified | Source Taxonomy, Coverage Decision Matrix | Bounded non-destructive kernel event windows are planned. |
| R-024 | Bounded local system event log windows are in scope where safe and available. | bounded | Source Taxonomy, Coverage Decision Matrix, Capture Planning Strategy | Journal/syslog files may be captured as bounded windows; structured journald parsing and command wrappers are not required for core capture. |
| R-025 | Service manager state is in scope only through native support. | deferred | Coverage Decision Matrix, Architecture | systemd/service manager data is not collected through `systemctl`. |
| R-026 | Netlink state is in scope. | specified | Source Taxonomy, Coverage Decision Matrix, Architecture | Link, address, route, neighbor, and other netlink families are native collection targets. |
| R-027 | Security and audit state is in scope. | bounded | Source Taxonomy, Coverage Decision Matrix | procfs/securityfs-visible state is collected; audit/log windows require native support. |
| R-028 | User and session runtime state is in scope. | bounded | Source Taxonomy, Coverage Decision Matrix | Runtime session metadata is collected where available without user-stream reads. |
| R-029 | Scheduler and job runtime state is in scope. | bounded | Source Taxonomy, Coverage Decision Matrix | Runtime state is considered; static schedule configuration is not primary evidence. |
| R-030 | Time synchronization runtime state is in scope (kernel-visible clocks). | bounded | Source Taxonomy, Coverage Decision Matrix | Local clock metadata via /proc and /etc; NTP/chrony protocol state is excluded. |
| R-031 | Crash dump metadata is in scope. | bounded | Source Taxonomy, Coverage Decision Matrix | Metadata and bounded records are captured; large core/vmcore payloads are not copied by default. |
| R-032 | Hardware management state is in scope when native safe access exists. | deferred | Source Taxonomy, Coverage Decision Matrix | Vendor CLI-dependent sources are `deferred-native`. |
| R-033 | Local container-related host-side evidence is in scope. | bounded | Source Taxonomy, Coverage Decision Matrix | Host-side procfs/cgroup/namespace evidence and runtime socket or directory metadata are collected; container daemon APIs and image contents are excluded. |
| R-034 | Kubernetes control-plane state is out of scope. | excluded | Project Design, Source Taxonomy, Coverage Decision Matrix | No Kubernetes API or orchestration control-plane collection is part of core capture. |
| R-035 | Cloud provider control-plane APIs are out of scope. | excluded | Project Design, Source Taxonomy, Coverage Decision Matrix | No cloud API collection belongs to core capture. |
| R-036 | Application configuration and business data are out of scope. | excluded | Project Design, Source Taxonomy, Coverage Decision Matrix | Core capture does not target business data or application config. |
| R-037 | Language runtime internals are out of scope. | excluded | Project Design, Coverage Decision Matrix | JVM, Go, Python, Node.js, .NET, BEAM, and similar runtime internals are not core targets. |
| R-038 | Full historical logs are out of scope. | excluded | Project Design, Coverage Decision Matrix | Only bounded event windows are valid capture targets. |
| R-039 | Snapshot format must support partial captures. | specified | Snapshot Specification, Safety and Operations | Missing `CAPTURE_COMPLETE` means partial evidence, not invalid evidence. |
| R-040 | Snapshot manifest records every meaningful object outcome. | specified | Snapshot Specification | Manifest uses the baseline status vocabulary. |
| R-041 | Errors are recorded without aborting the whole capture. | specified | Project Design, Snapshot Specification, Safety and Operations | Expected runtime races are represented as object outcomes. |
| R-042 | The design must distinguish skipped, limited, truncated, unsupported, and failed objects. | specified | Snapshot Specification, Coverage Decision Matrix | Object statuses and coverage decisions have separate meanings. |
| R-043 | Output must be treated as sensitive. | specified | Safety and Operations | Snapshot directories use restrictive permissions and are handled as sensitive artifacts. |
| R-044 | Collection must be non-destructive. | specified | Safety and Operations | Collectors do not write to runtime control interfaces or mutate services/processes. |
| R-045 | Capture work must be bounded. | specified | Capture Planning Strategy, Safety and Operations | Every task has timeout, byte, file, depth, and concurrency limits. |
| R-046 | The design must explain implementation phases. | specified | Implementation Plan | Milestones define delivery order without duplicating taxonomy decisions. |
| R-047 | The design must preserve architectural decisions. | specified | ADR | Project-level decisions are recorded and reviewed before change. |
| R-048 | The design must include full coverage decisions. | specified | Coverage Decision Matrix | Every taxonomy source family has explicit collection decisions and rationales. |
| R-049 | Snapshot control files must have explicit schema and compatibility rules. | specified | Snapshot Specification | Manifest, plan, and error records define required fields, path rules, and schema evolution behavior. |
| R-050 | Coverage decisions, plan decisions, and manifest statuses must remain distinct and mapped. | specified | Snapshot Specification, Capture Planning Strategy, Coverage Decision Matrix, Glossary | Static intent, planner scheduling, and final object outcomes use separate vocabularies with an explicit mapping. |
| R-051 | AI-assisted development must follow canonical project instructions. | specified | AGENTS, AI Context Index, AI Development Guide, AI Skill Specs, AI Prompt Library | AI tools have a single project entry point, task-specific context routing, reusable skill specs, and validation expectations. |
| R-052 | Code implementation must wait for documentation readiness. | specified | Documentation Readiness Gate, Design Review Checklist, ADR, Implementation Plan, AGENTS | Milestone 1 does not begin until non-code documents pass readiness and review gates with no blockers. |
| R-053 | AI-assisted work must stay inside this repository unless explicitly requested otherwise. | specified | AGENTS, Documentation Readiness Gate, AI Development Guide, ADR, CLAUDE, Copilot Instructions | AI tools do not edit or propose changes outside the repository without an explicit user request. |
| R-054 | Capture must preserve a complete, comprehensive, and effective Linux runtime raw snapshot within the configured collection budget. | bounded | Project Design, Capture Planning Strategy, Safety and Operations, Snapshot Specification, Glossary | All in-scope source families are captured or explicitly accounted for, and no capture task exceeds configured time, byte, file, depth, or concurrency limits to improve apparent completeness. |

## Acceptance Gates Before Implementation

Before code implementation begins, the design is considered ready only if:

- Every explicit requirement maps to at least one canonical document.
- Every source family in the taxonomy maps to coverage decisions.
- Every non-collected source has a reason.
- No collector decision depends on invoking an external command.
- Root-only language applies to `capture`, not offline artifact commands.
- Unsafe-code policy is unambiguous.
- Snapshot completion and partial-capture semantics are defined.
- Planner decisions are inspectable after capture.
- Schema compatibility and unknown-field behavior are defined.
- Coverage decision, plan decision, and manifest status vocabularies are mapped.
- AI-assisted development instructions and context routing are present.
- Documentation Readiness Gate and Design Review Checklist pass with no blocker.
- Repository-local work boundaries are documented for AI-assisted work.

## Change Control

Any future design change should update this matrix when it changes:

- Project scope.
- Safety policy.
- Collector eligibility.
- Coverage decision status.
- Snapshot format semantics.
- Planner behavior.
- Implementation milestone boundaries.
