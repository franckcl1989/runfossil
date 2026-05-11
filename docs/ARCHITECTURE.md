# Architecture

## Overview

`runfossil` uses a Rust workspace with a small command-line entry point and
separate internal crates for planning, collection, storage, packaging, and shared
types. The architecture favors narrow module responsibilities so new source
families can be added without changing the capture engine.

This document describes component boundaries and data flow. It intentionally
avoids concrete Rust trait or function signatures until the design phase is
complete.

## Workspace Shape

The intended workspace structure is:

```text
crates/
  runfossil-cli
  runfossil-core
  runfossil-plan
  runfossil-store
  runfossil-fs
  runfossil-proc
  runfossil-net
  runfossil-service
  runfossil-container
  runfossil-pack
```

The exact crate list can evolve, but the ownership boundaries should remain
stable.

## Component Responsibilities

### runfossil-cli

Responsibilities:

- Parse the small command surface.
- Enforce root-only execution before capture.
- Create a capture session.
- Report final exit status.
- Keep user-facing behavior stable.

Non-responsibilities:

- Source-specific collection logic.
- Snapshot layout rules.
- Planning heuristics.
- Long diagnostic output.

### runfossil-core

Responsibilities:

- Shared domain model.
- Snapshot identifiers and host identifiers.
- Source, domain, object, task, and outcome concepts.
- Error categories.
- Budget concepts.
- Time and byte accounting concepts.
- Project-wide invariants.

`runfossil-core` should not know how to collect a specific source. It defines
the language used by the rest of the workspace.

### runfossil-plan

Responsibilities:

- Probe host capabilities.
- Estimate host scale and pressure.
- Build the capture task graph.
- Assign priority, cost, risk, and budgets.
- Record planner decisions for `plan.json`.
- Adapt when sources are unavailable or expensive.

The planner decides what should be attempted. It does not write snapshot files
directly and does not parse raw data except where required for planning.

### runfossil-store

Responsibilities:

- Create the snapshot directory.
- Write raw object files.
- Write directory listings, symlink targets, and metadata records.
- Record manifest entries.
- Append error events.
- Use atomic write patterns.
- Mark capture completion.

The store owns the on-disk format. Collectors produce bytes and metadata; the
store decides how those results are represented in the snapshot directory.

### runfossil-fs

Responsibilities:

- Generic virtual filesystem collection.
- Safe bounded file reads.
- Directory traversal with depth and count limits.
- Symlink target capture.
- File metadata capture.
- Common `/proc`, `/sys`, `/run`, and `/dev` path handling primitives.

This crate provides reusable building blocks for source-specific collectors.

### runfossil-proc

Responsibilities:

- Global `/proc` runtime files.
- Process and thread enumeration.
- Process status, stats, command line, environment, cgroup, namespace, fd, maps,
  smaps, scheduler, I/O, and limit data.
- Handling normal process races such as PID reuse and process exit.

The crate should prefer preserving raw procfs content over interpreting it.

### runfossil-net

Responsibilities:

- Native netlink collection where supported.
- `/proc/net` raw fallback and supplement collection.
- Link, address, route, neighbor, socket diagnostic, conntrack, traffic control,
  and XFRM state.

The crate must not invoke `ip`, `ss`, `tc`, `conntrack`, or similar commands.

### runfossil-service

Responsibilities:

- Service manager state through native protocol support.
- systemd-oriented manager, unit, service, socket, timer, failed, and degraded
  state.
- logind and time synchronization service state when implemented natively.

The crate must not invoke `systemctl`, `loginctl`, `timedatectl`, or
`journalctl`.

### runfossil-container

Responsibilities:

- Local container runtime discovery.
- Native socket/protocol collection for Docker, containerd, CRI-O, and runc
  state where feasible.
- Container object, process, resource, namespace, cgroup, event, and log-window
  evidence.

The crate must not invoke `docker`, `ctr`, `crictl`, `podman`, or `runc`
commands.

### runfossil-pack

Responsibilities:

- Package a completed snapshot directory.
- Compress after capture.
- Preserve manifest and raw files without changing evidence content.
- Support future archive verification.

Packaging is never required for incident-time evidence preservation.

## Capture Data Flow

The capture flow is:

```text
CLI
  -> root check
  -> create session metadata
  -> planner probe
  -> planner task graph
  -> store creates snapshot directory
  -> executor runs bounded tasks
  -> collectors read runtime sources
  -> store persists raw results and outcomes
  -> manifest and plan are finalized
  -> CAPTURE_COMPLETE is written
```

The store should be initialized before expensive collection begins so partial
evidence survives if the process is interrupted.

## Execution Model

The executor should support concurrency, but bounded concurrency is mandatory.
Collection is mostly I/O-bound and can damage an already stressed host if it
fans out without limits.

Execution should respect:

- Global capture budget.
- Per-source budget.
- Per-task timeout.
- Per-object byte limit.
- Per-directory file count.
- Per-directory depth limit.
- Dynamic backpressure from observed errors and slow reads.

The executor should favor high-priority low-cost tasks first. Expensive tasks
should run only after core evidence has been persisted.

## Data Ownership

Collectors own source-specific access. The store owns persistence. The planner
owns task selection. The CLI owns user-facing flow. No source-specific collector
should encode snapshot directory layout beyond stable object identity.

## Dependency Direction

The intended dependency direction is:

```text
runfossil-cli        -> runfossil-core
runfossil-cli        -> runfossil-plan
runfossil-cli        -> runfossil-store
runfossil-cli        -> collector crates
runfossil-cli        -> runfossil-pack

runfossil-plan       -> runfossil-core
runfossil-store      -> runfossil-core
runfossil-fs         -> runfossil-core
collector crates     -> runfossil-core
collector crates     -> runfossil-fs when filesystem helpers are needed
runfossil-pack       -> runfossil-core
runfossil-pack       -> runfossil-store when snapshot layout helpers are needed
```

Circular dependencies are not allowed.

## Raw-First Boundary

Parsing is allowed for:

- Discovering object lists.
- Estimating cost.
- Detecting host capabilities.
- Deciding whether to deepen collection.
- Producing stable metadata such as byte counts, timestamps, and task outcomes.

Parsing is not the main product output. Parsed summaries should not replace raw
evidence in the initial release.

## Platform Boundary

The project targets Linux. Cross-platform abstraction should not obscure Linux
runtime semantics. Conditional compilation is acceptable where it keeps Linux
support precise or allows non-Linux builds to fail cleanly.

## Extensibility Model

New source families should be added by:

1. Extending the source taxonomy if the family is not already covered.
2. Adding planner probes and task generation.
3. Implementing a bounded collector.
4. Adding manifest outcomes and test fixtures.
5. Updating documentation for source coverage and limitations.

New collectors must not introduce external command dependencies.
