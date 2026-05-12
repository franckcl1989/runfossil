# Risk Register

## Purpose

This document records design risks that can affect correctness, production
safety, forensic value, implementation complexity, or future maintainability.

The register is not a blocker list. It defines known risks, mitigation strategy,
and the design document that controls each risk.

## Severity Vocabulary

```text
critical  Can make the tool unsafe for production or destroy evidence.
high      Can significantly reduce forensic value or reliability.
medium    Can cause partial coverage, performance cost, or implementation delay.
low       Needs tracking but does not threaten the core design.
```

## Status Vocabulary

```text
controlled  The current design has a mitigation.
open        The risk is known but needs further design before implementation.
accepted    The risk is inherent and handled by documentation or operator policy.
```

## Risks

| ID | Risk | Severity | Status | Mitigation | Control document |
|---|---|---:|---:|---|---|
| RK-001 | Capture mutates production runtime state. | critical | controlled | Read-only policy, no writes to runtime control interfaces, no process control. | Safety and Operations |
| RK-002 | Capture uses external commands with unstable behavior. | high | controlled | External command collection is prohibited. Deferred-native is used instead. | ADR, Safety and Operations, Coverage Decision Matrix |
| RK-003 | Unsafe project-owned code runs as root. | critical | controlled | Every project crate must use `#![forbid(unsafe_code)]`. | Project Design, Safety and Operations, ADR |
| RK-004 | A single slow or large source blocks the entire snapshot. | high | controlled | Per-task timeout, byte, file, depth, and concurrency budgets. | Capture Planning Strategy, Safety and Operations |
| RK-005 | High pressure host is made worse by deep traversal. | high | controlled | Planner reduces deep work under pressure and prioritizes P0/P1 evidence. | Capture Planning Strategy |
| RK-006 | Partial capture is misinterpreted as complete. | high | controlled | `CAPTURE_COMPLETE` is the completion marker; manifest records outcomes. | Snapshot Specification |
| RK-007 | Runtime objects disappear during capture. | medium | accepted | `vanished` is a normal status and is recorded. | Snapshot Specification, Safety and Operations |
| RK-008 | Snapshot contains secrets or sensitive operational data. | high | accepted | Treat snapshots as sensitive, use restrictive permissions, avoid default redaction. | Safety and Operations |
| RK-009 | Redaction destroys forensic value. | medium | accepted | Raw evidence is preserved by default; redaction is future offline tooling. | Project Design, Safety and Operations |
| RK-010 | Large core, vmcore, or kernel memory payloads consume storage or expose sensitive memory. | high | controlled | Metadata-only by default; payload copying is excluded from core capture. | Coverage Decision Matrix |
| RK-011 | `/dev` reads block or have side effects. | critical | controlled | Device nodes are metadata-only except explicitly bounded event sources. | Coverage Decision Matrix, Safety and Operations |
| RK-012 | DebugFS, TraceFS, or BPF collection changes kernel behavior. | high | controlled | Do not enable tracing; collect only existing state under strict limits. | Coverage Decision Matrix, Safety and Operations |
| RK-013 | service/log/journal support depends on command wrappers. | medium | controlled | Excluded from kernel-focused scope; file-based bounded journal window capture is handled separately. | Coverage Decision Matrix |
| RK-014 | Hardware management requires vendor CLIs or C SDKs. | medium | controlled | Excluded from kernel-focused scope; sysfs sensor/thermal/hwmon state is captured. | Coverage Decision Matrix |
| RK-015 | Container runtime state is incomplete before native runtime protocols exist. | medium | controlled | Capture host-side procfs, namespace, and cgroup evidence; container daemon APIs are excluded from kernel-focused scope. | Coverage Decision Matrix |
| RK-016 | Planner decisions are hard to explain after the fact. | high | controlled | Persist planner probes, budgets, priorities, and skip reasons in `plan.json`. | Snapshot Specification, Capture Planning Strategy |
| RK-017 | Snapshot layout changes break offline tools. | high | controlled | Treat Snapshot Specification as a compatibility contract. | Snapshot Specification, ADR |
| RK-018 | Taxonomy and coverage decisions diverge. | medium | controlled | Source Taxonomy owns source boundaries; Coverage Decision Matrix owns decisions. | Source Taxonomy, Coverage Decision Matrix |
| RK-019 | Implementation plan duplicates coverage policy and drifts. | medium | controlled | Implementation Plan owns delivery order only. | Implementation Plan, Source Taxonomy |
| RK-020 | Dependency risk enters through low-level Linux crates. | medium | controlled | Use conservative dependency policy, justify unsafe transitive dependencies, and avoid command-wrapper crates. | Safety and Operations |
| RK-021 | Kernel version and distribution differences reduce coverage. | medium | accepted | Missing sources become `not_found` or `unsupported`, not fatal errors. | Snapshot Specification, Coverage Decision Matrix |
| RK-022 | Filesystem output path causes self-capture or recursion. | high | controlled | Validate output path and avoid placing it inside captured source trees. | Safety and Operations |
| RK-023 | Output filesystem fills during capture. | high | controlled | Validate output path, check available space, stop low-priority work first, and preserve partial evidence. | Safety and Operations |
| RK-024 | Hashing large payloads delays incident-time capture. | medium | controlled | Hashes are recommended but may be skipped for large objects by policy. | Snapshot Specification |
| RK-025 | Ambiguous terminology causes implementation drift. | medium | controlled | Glossary defines core terms. | Glossary |
| RK-026 | Ambiguous snapshot schema breaks offline tooling or future compatibility. | high | controlled | Snapshot Specification defines required fields, path rules, schema evolution, and unknown-field behavior. | Snapshot Specification |
| RK-027 | Coverage decisions, planner decisions, and manifest statuses drift apart. | medium | controlled | Static intent, plan decisions, and final outcomes have separate vocabularies and an explicit mapping. | Snapshot Specification, Capture Planning Strategy, Coverage Decision Matrix |
| RK-028 | AI-assisted changes invent behavior, skip context, or drift from canonical design. | medium | controlled | AGENTS, AI Context Index, AI Development Guide, AI Skill Specs, and AI Prompt Library define required context, workflows, and validation. | AGENTS, AI Context Index, AI Development Guide |
| RK-029 | Implementation starts before documentation contracts are ready. | high | controlled | Documentation Readiness Gate and Design Review Checklist block Milestone 1 until non-code documents are complete and consistent. | Documentation Readiness Gate, Design Review Checklist, Implementation Plan, ADR |
| RK-030 | AI-assisted work changes files outside the repository scope. | medium | controlled | AGENTS, Documentation Readiness Gate, AI Development Guide, Claude instructions, and Copilot instructions require repository-local work unless explicitly requested otherwise. | AGENTS, Documentation Readiness Gate, AI Development Guide |
| RK-031 | AI-generated code introduces unsafe Rust blocks despite project policy. | high | controlled | Workspace-wide `unsafe_code = "forbid"` lint rejects unsafe code at compile time. `clippy.toml` disallows `Command` types. CI enforcement prevents merges. | Cargo.toml workspace lints, clippy.toml, Safety and Operations |
| RK-032 | AI hallucinates collector implementations that invoke external commands. | high | controlled | `clippy.toml` disallows `std::process::Command` and `Command::new`. Collectors must use native Rust interfaces. AI skill specs and prompt library prohibit external-command workarounds. | clippy.toml, Safety and Operations, AI Skill Specs |
| RK-033 | AI changes vocabulary or terminology without updating dependent documents. | medium | controlled | AGENTS.md change rules require dependent-document updates. AI Context Index routes vocabulary tasks to Glossary. AI Skill Specs require cross-document validation. | AGENTS.md, AI Context Index, AI Skill Specs |
| RK-034 | AI fails to run validation commands after making changes. | medium | controlled | AGENTS.md Validation section lists mandatory commands. AI Development Guide requires validation reporting. AI Prompt Library prompts include validation steps. | AGENTS.md, AI Development Guide, AI Prompt Library |
| RK-035 | AI introduces code that contradicts design baseline behavior. | high | controlled | AGENTS.md requires reading specific design documents before changes. Non-Negotiable Constraints section defines inviolable rules. AI Development Guide requires canonical-document alignment. | AGENTS.md, AI Development Guide, AI Context Index |
| RK-036 | Multiple AI tools produce conflicting interpretations of the same design document. | medium | controlled | AGENTS.md is the single canonical AI instruction entry point. Tool-specific files are required to defer to AGENTS.md. Glossary defines terms for all tools. AI Skill Specs define shared workflows. | AGENTS.md, CLAUDE.md, Copilot Instructions, Glossary, AI Skill Specs |
| RK-037 | Signal handling regresses to project-owned unsafe code or hides cancellation behavior. | medium | controlled | SIGINT/SIGTERM cancellation uses safe `nix` signal mask and `sigwait` APIs in project code. The dependency is pinned in `Cargo.lock`, audited by cargo-deny, and cancellation preserves a partial snapshot instead of finalizing incomplete capture as complete. | Safety and Operations, Implementation Plan, Release Readiness |
| RK-038 | Release binaries depend on host glibc or libgcc runtime libraries. | high | controlled | CI builds `x86_64-unknown-linux-musl` release artifacts, verifies static linkage with `file` and `ldd`, and publishes checksums with the artifact. | Release Readiness, CI |
| RK-039 | Release validation claims exceed environments actually tested. | medium | controlled | Release Readiness separates automated gates from root production-like capture, cross-distribution compatibility, benchmark runs, and signing-key validation that require external release environments. | Release Readiness, Implementation Plan |

## Risk Review Rules

Risk entries should be updated when:

- A coverage decision changes.
- A source moves between collect, conditional, limited, deferred-native, and
  exclude.
- A new native protocol collector is introduced.
- A new dependency with meaningful privilege, parsing, or unsafe-code risk is
  added.
- Snapshot format semantics change.
- Planner budget or priority rules change.
- Schema compatibility rules or decision/status vocabularies change.
- AI workflow, skill specs, or prompt guidance change.
- Documentation readiness or implementation-entry policy changes.
- Repository-local work policy changes.

## Pre-Implementation Risk Gate

Before implementation starts, these risks must be controlled by design:

- RK-001
- RK-002
- RK-003
- RK-004
- RK-006
- RK-011
- RK-016
- RK-017
- RK-026
- RK-027
- RK-028
- RK-029
- RK-030
- RK-031
- RK-032
- RK-035

The current design controls these risks through documented policy and artifact
contracts. Implementation must preserve those controls.
