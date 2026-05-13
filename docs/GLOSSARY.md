# Glossary

## Purpose

This glossary defines project terminology used across the `runfossil` design
documents. Terms in this file should be used consistently in documentation and
future implementation.

## Terms

### AGENTS.md

The canonical AI agent instruction file at the repository root. It is the
authoritative entry point for any AI coding agent working on this repository.
All other AI tool entry points must defer to `AGENTS.md`.

### Agents Directory (.agents/)

A repository directory reserved for agent-specific configuration files and
workflows. Content in `.agents/` should remain compatible with the constraints
in `AGENTS.md`.

### AI-Assisted Development

Development work performed or assisted by AI coding agents, subject to the same
design, safety, snapshot, and validation constraints as any other work. AI
assistance does not exempt changes from documentation readiness, review
checklist, vocabulary consistency, or traceability requirements.

### Artifact Path

A relative path inside a snapshot directory. Artifact paths must not be
absolute, must not contain `..` components, and must not collide with reserved
format-owned names. They are the stable addresses of evidence within a snapshot.

### Budget (Capture)

The set of limits assigned to a capture task or the global capture session.
Budgets include timeout, maximum bytes, maximum files, maximum depth, and
concurrency class. Budgets are recorded in `plan.json` and `meta/limits.json`.

### Canonical Document

The single document that owns and defines a specific design topic.
Cross-references may summarize, but only the canonical document is
authoritative for its topic. See the canonical ownership table in
`docs/DOCUMENTATION_READINESS_GATE.md`.

### CLAUDE.md

The Claude-specific AI instruction file at the repository root. It delegates to
`AGENTS.md` for all project policy and should remain a thin adapter unless
Claude Code needs tool-specific behavior.

### Codex Directory (.codex/)

A repository directory reserved for Codex-specific configuration, skills, and
workflows. Content in `.codex/` must not contradict `AGENTS.md`, project safety
policy, or the design baseline.

### Collection Mode

The storage or collection method assigned to a coverage unit. Baseline modes
include `raw-file`, `raw-file-set`, `dir-listing`, `symlink-targets`,
`metadata`, `bounded-tree`, `bounded-window`, `native-protocol`,
`native-netlink`, `metadata-only`, and `skip`.

### Copilot Instructions

The GitHub Copilot instruction file at `.github/copilot-instructions.md`. It
delegates to `AGENTS.md` for project policy.

### Crate

A single Rust compilation unit in the workspace. The workspace contains ten
crates: one binary (`runfossil-cli`) and nine libraries covering core models,
planning, storage, filesystem helpers, and source-specific collectors.

### Capture

The act of collecting live Linux runtime evidence from the current host.
`runfossil capture` is root-only.

### Capture Objective

The project target for `runfossil capture`: preserve a complete, comprehensive,
and effective raw snapshot of in-scope Linux operating-system runtime evidence
for offline incident reconstruction, without exceeding the configured
collection budget.

### Snapshot

The directory artifact produced by capture. A snapshot contains raw evidence,
metadata, a manifest, a plan record, error records, and optionally a completion
marker.

### Snapshot Control File

A machine-readable file inside a snapshot directory that follows an explicit
schema. Snapshot control files include `manifest.json`, `plan.json`, and
`errors.jsonl`. Their schemas and compatibility rules are defined in the
Snapshot Specification and are treated as contracts between the capture
implementation and offline analysis tools.

### Skeleton

A minimal initial implementation that establishes crate structure, key types,
public APIs, and basic test coverage without implementing the full feature set.
Skeletons allow project-wide constraints (linting, formatting, safety policy)
to be enforced from the start. For the current implementation status, see the
Implementation Plan.

### Raw Evidence

Runtime data preserved with minimal interpretation. Examples include raw file
contents, directory listings, symlink targets, metadata records, event windows,
and native protocol responses.

### Raw Snapshot Object

A specific evidence object from the source taxonomy. It can be a file, directory
listing, symlink family, native response, metadata record, or event window.

### Reserved Name

A format-owned file name inside a snapshot directory that must not be used for
raw object paths. Reserved names include `_content`, `_listing.json`,
`_metadata.json`, and `_symlink`. Their use must be documented in manifest
entries.

### Source

The L1 root collection source in the taxonomy, such as `/proc`, `/sys`, netlink,
kernel ring buffer, or service manager.

### Source Slug

The stable lowercase identifier for an L1 source family in snapshot control
files, such as `proc`, `sys`, `netlink`, `kernel`, or `service`.

### Workspace

The Cargo workspace defined by the root `Cargo.toml`. It contains separate
crates for CLI, core models, planning, storage, filesystem helpers,
source-specific collectors, and packaging. The workspace enforces shared
linting, formatting, and safety policies across all member crates.

### Domain

The L2 internal source domain under an L1 source, such as process state under
`/proc`, cgroups under `/sys`, or route state under netlink.

### Effective UID

The user identity observed by the capture entry point, read from
`/proc/self/status`. An effective UID of zero means root. The capture command
must fail early when the effective UID is not zero. Permission failures by
kernel or LSM policy can still occur even with an effective UID of zero.

### Error Log

`errors.jsonl`, the append-only event stream for capture errors and notable
runtime races.

### Completion Marker

`CAPTURE_COMPLETE`, written only after capture metadata is finalized. If missing,
the snapshot is partial.

### Manifest Status

A final object outcome recorded in `manifest.json`, such as `captured`,
`vanished`, `not_found`, `permission_denied`, `timeout`, `size_limited`,
`truncated`, `skipped_by_policy`, `unsupported`, or `io_error`.

### Milestone

A numbered delivery phase in the Implementation Plan. Each milestone has
specific deliverables and exit criteria. Milestones 0 through 9 are currently
defined. Milestones 0 through 8 have baseline implementations recorded as
complete or implemented, and Milestone 9 hardening and release-readiness work is
in progress.

### Partial Snapshot

A snapshot directory without `CAPTURE_COMPLETE`, or one whose manifest indicates
incomplete capture. Partial snapshots remain valid evidence.

### Snapshot Completeness

The accountable state of a snapshot, not a promise that every possible runtime
byte was copied. A complete capture records final metadata and accounts for
in-scope evidence through outcomes such as `captured`, `limited`,
`skipped_by_policy`, `unsupported`, `not_present`, `not_found`, `timeout`,
`size_limited`, `truncated`, `io_error`, or related manifest and planner
outcomes.

### Root-Only Capture

The policy that live capture requires effective UID zero. Offline commands such
as packaging and inspection operate on existing artifacts and do not require
root by design.

### Project-Owned Safe Code

Rust code in this repository. It must use `#![forbid(unsafe_code)]`.

### Native Collection

Collection through Rust code using Linux interfaces such as virtual filesystems,
netlink, Unix sockets, D-Bus protocol support, journald file or socket support,
or runtime APIs implemented in Rust.

### External Command

A process launched to obtain collection data, such as `ip`, `ss`, `systemctl`,
`journalctl`, `docker`, `smartctl`, `nvme`, or `ipmitool`. External command
collection is prohibited.

### Bounded Collection

Collection with explicit limits such as timeout, maximum bytes, maximum files,
maximum depth, selected objects, event-window size, or metadata-only mode.

### Event Window

A bounded slice of runtime events from kernel, system log, service, audit, or
security sources.

### Executor

The runtime component that runs the capture task graph produced by the planner.
The executor respects global and per-task budgets, enforces concurrency limits,
applies dynamic backpressure, and records execution feedback that can reduce
further work under pressure.

### Host Probe

A low-cost fact-gathering operation executed during the planner's probe phase.
Examples include reading the effective UID, hostname, kernel release, PSI
pressure files, detecting cgroup version, counting processes and mounts, and
checking for container daemon sockets. Probes must be bounded and must not
deeply traverse dynamic trees.

### Incident Signal

A runtime indicator detected during planning or execution that justifies deeper
collection in a specific source family. Examples include OOM messages, hung-task
reports, filesystem errors, device resets, service failures, and
container-related crash indicators. Incident signals deepen the relevant source
family without switching the entire capture into a heavy mode.

### Metadata-Only

A collection mode that records existence, type, permissions, size, timestamp,
identity, symlink target, major/minor number, or other metadata without copying
large or risky content.

### Deferred Native

A valid target that must wait until a Rust-native implementation exists.
External command output is not an acceptable temporary replacement.

### Documentation-First Policy

The rule that code implementation must not begin until non-code project
documentation passes the Documentation Readiness Gate and Design Review
Checklist with no blockers. This policy applies to both human and AI-assisted
development. When implementation discovers a contract gap, documentation must
be updated before the behavior is treated as accepted.

### Not Present

A planner decision meaning probing showed that a source or object is absent on
this host. The corresponding manifest status, when an important object is
represented, is `not_found`.

### Unsupported

A source exists but the current implementation cannot collect it natively.
Unsupported is a runtime object outcome and must be explicit in the manifest.

### Schema Version

The integer version attached to machine-readable snapshot control files. It
defines how readers interpret required fields and compatibility rules.

### Skipped By Policy

The planner intentionally did not collect an object because of cost, risk,
pressure, scale, source overlap, or release policy.

### Size Limited

The object was not captured as content because its estimated or observed size
exceeded policy limits.

### Truncated

Partial content was intentionally written within policy limits.

### Vanished

The object existed during discovery but disappeared before or during capture.
This is normal for runtime state such as processes, file descriptors, sockets,
cgroups, and container-related host objects.

### Incident-Time Path

The work performed while production recovery is waiting. Incident-time work must
be fast, bounded, non-destructive, and focused on evidence preservation.

### Object Kind

The type of a raw snapshot object recorded in `manifest.json`. Baseline object
kinds include `file`, `file_set`, `dir_listing`, `symlink`, `metadata`,
`metadata_only`, `bounded_tree`, `event_window`, and `native_dump`. Object kind
is independent from collection mode; it describes the artifact, not the method.

### Offline Path

Work performed after evidence is preserved, such as packaging, inspection,
validation, summarization, or redaction.

### Documentation Readiness Gate

The pre-implementation review gate that requires every non-code project document
to be complete, consistent, linked, traceable, and validated before code
implementation begins.

### Repository-Local Work

AI-assisted work limited to files in this repository unless the user explicitly
asks for project-external changes.

### AI Agent Instructions

Repository-owned guidance for AI coding agents. `AGENTS.md` is the canonical
entry point for AI-assisted repository work.

### AI Context Index

The document that maps task types to the smallest useful set of canonical project
documents for AI-assisted work.

### AI Skill Spec

A project-owned specification for a reusable AI workflow. A skill spec may later
be converted into an installed tool-specific skill, but this repository document
remains the project source of truth.
