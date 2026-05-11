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
| RK-013 | service/log/journal support depends on command wrappers. | medium | controlled | Mark as deferred-native until Rust-native support exists. | Coverage Decision Matrix |
| RK-014 | Hardware management requires vendor CLIs or C SDKs. | medium | controlled | Mark as deferred-native until safe native design exists. | Coverage Decision Matrix |
| RK-015 | Container runtime state is incomplete before native runtime protocols exist. | medium | controlled | Capture host-side procfs, namespace, and cgroup evidence; defer runtime APIs. | Coverage Decision Matrix |
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

The current design controls these risks through documented policy and artifact
contracts. Implementation must preserve those controls.
