# Snapshot Specification

## Goals

This specification defines the on-disk evidence package produced by
`runfossil capture`. It is the compatibility contract between the capture
implementation and offline analysis tools.

The format must be:

- Useful when opened as a normal directory.
- Stable enough for offline tooling.
- Resilient to interrupted capture.
- Explicit about missing, skipped, truncated, and failed objects.
- Independent from compression or archive packaging.

## Contract Language

This document uses contract language deliberately:

- `must` means an implementation or offline tool is non-conforming if it does not
  follow the rule.
- `should` means the rule is expected unless a documented implementation
  constraint or incident-time safety concern prevents it.
- `may` means the behavior is optional and must not be required by readers.

The snapshot directory is the canonical artifact. Archive formats, inspection
summaries, redaction output, and parsed analysis are derived artifacts.

A complete snapshot means final metadata and the completion marker were written.
It does not mean every possible Linux runtime byte was copied. Forensic
completeness is represented by raw evidence plus explicit plan and manifest
outcomes such as `captured`, `not_found`, `not_present`,
`skipped_by_policy`, `limited`, `timeout`, `size_limited`, `truncated`, or
`io_error`.

## Schema and Compatibility

All machine-readable control files must be UTF-8 JSON unless this specification
names another encoding. JSON files must not depend on comments, trailing commas,
or field ordering.

Compatibility rules:

- `schema_version` is a positive integer.
- Readers must reject a `schema_version` they do not understand.
- Readers should ignore unknown fields in known objects.
- Tools that rewrite control files should preserve unknown fields when practical.
- New optional fields may be added without changing `schema_version`.
- Removing a required field or changing a required field's meaning requires a new
  schema version.
- Raw evidence files are never rewritten for schema migration.

Timestamp fields ending in `_unix_ns` are decimal strings containing nanoseconds
since the Unix epoch in UTC. They are strings so readers do not lose precision in
JSON environments with limited integer ranges.

Count, byte, depth, timeout, and elapsed fields are non-negative JSON numbers
unless this specification explicitly defines them as strings.

## Identifiers and Paths

Object IDs must be stable ASCII identifiers using lowercase letters, digits, and
dots. They should follow:

```text
<source>.<domain>.<object-name>
```

Example:

```text
proc.system.stat
proc.process.123.status
netlink.route.dump
```

Canonical source slugs:

| Source family | Slug |
|---|---|
| `/proc` | `proc` |
| `/sys` | `sys` |
| `/run` | `run` |
| `/dev` | `dev` |
| Kernel ring buffer | `kernel` |
| System event log store | `logs` |
| Service manager | `service` |
| Netlink | `netlink` |
| Security / audit subsystem | `security` |
| User / session database | `sessions` |
| Scheduler / job runtime | `scheduler` |
| Time synchronization subsystem | `time` |
| Crash dump store | `crash` |
| Hardware management interface | `hardware` |

Manifest `path` values must be relative paths inside the snapshot directory. They
must not be absolute paths and must not contain `..` components.

Runtime source paths should be mirrored under `raw/` when the source path has
safe UTF-8 path components. If a runtime path component is not safe to mirror,
the store must escape it deterministically and record the original source path in
the manifest object entry. A safe path component:

- is valid UTF-8;
- is not empty;
- is not `.` or `..`;
- does not contain `/` or NUL;
- does not collide with reserved format-owned names.

When escaping is required, the escaped output path remains the artifact path, and
the manifest keeps enough metadata for offline tools to reconstruct the original
source identity.

## Directory Layout

The snapshot directory name should include UTC time, host identity, and boot
identity:

```text
snapshot-<utc-time>-<hostname>-boot-<boot-id-short>/
```

Example:

```text
snapshot-20260509T121530Z-prod-db-01-boot-3f2a9b7c/
```

The baseline layout is:

```text
snapshot-.../
  manifest.json
  plan.json
  errors.jsonl
  meta/
    host.json
    runtime.json
    capabilities.json
    limits.json
  raw/
    proc/
    sys/
    run/
    dev/
    netlink/
    kernel/
    logs/
    service/
    security/
    sessions/
    scheduler/
    time/
    crash/
    hardware/
  CAPTURE_COMPLETE
```

`CAPTURE_COMPLETE` is written last. If it is missing, offline tooling must treat
the snapshot as partial.

## Atomicity Rules

- The snapshot directory is created before collection begins.
- Metadata and raw files are written through temporary files and then renamed
  into place.
- `errors.jsonl` may be append-only during capture.
- `manifest.json` and `plan.json` may be written initially as temporary
  versions, but their final versions must be atomically replaced.
- `CAPTURE_COMPLETE` is created only after final metadata is durable.

The format should remain inspectable if the process is killed mid-capture.

## Raw Path Mapping

Runtime paths should be mirrored under `raw/` where practical:

```text
/proc/stat              -> raw/proc/stat
/proc/123/status        -> raw/proc/123/status
/sys/class/net/eth0/mtu -> raw/sys/class/net/eth0/mtu
/run/systemd            -> raw/run/systemd/
```

For non-filesystem sources, use stable logical paths:

```text
netlink/link.dump
netlink/route.dump
kernel/kmsg.window
logs/system.window
service/systemd/units.dump
sessions/logind/sessions.dump
```

If a source path conflicts because it can be both a file-like object and a
directory-like object, reserve an internal file name:

```text
<path>/_content
<path>/_listing.json
<path>/_metadata.json
<path>/_symlink
```

These reserved names are format-owned and should be documented in manifest
entries when used.

## Manifest

`manifest.json` is the authoritative index for a snapshot. It should not contain
large raw payloads.

Required top-level fields:

```json
{
  "schema_version": 1,
  "tool": "runfossil",
  "tool_version": "0.0.0",
  "started_at_unix_ns": "1778328930000000000",
  "finished_at_unix_ns": "1778328934123000000",
  "complete": true,
  "root": true,
  "host": {
    "hostname": "prod-db-01",
    "boot_id": "3f2a9b7c-0000-0000-0000-000000000000",
    "kernel_release": "6.8.0-xx-generic",
    "machine": "x86_64"
  },
  "objects": []
}
```

Top-level field contract:

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `schema_version` | number | yes | Snapshot manifest schema version. |
| `tool` | string | yes | Producing tool name. |
| `tool_version` | string | yes | Producing tool version. |
| `started_at_unix_ns` | string | yes | Capture start timestamp. |
| `finished_at_unix_ns` | string or null | yes | Capture finish timestamp, or null for an unfinished manifest. |
| `complete` | boolean | yes | Whether final metadata and `CAPTURE_COMPLETE` were written. |
| `root` | boolean | yes | Whether live capture ran with effective UID zero. |
| `host` | object | yes | Host identity summary. |
| `objects` | array | yes | Manifest object entries. |

Example object entry:

```json
{
  "id": "proc.system.stat",
  "source": "proc",
  "domain": "system",
  "object": "/proc/stat",
  "kind": "file",
  "status": "captured",
  "path": "raw/proc/stat",
  "bytes": 4096,
  "hash": {
    "algorithm": "sha256",
    "value": "..."
  },
  "started_at_unix_ns": "1778328930123000000",
  "finished_at_unix_ns": "1778328930124000000",
  "elapsed_us": 100,
  "limits": {
    "max_bytes": 1048576,
    "timeout_ms": 50
  },
  "reason": null
}
```

Object entry field contract:

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `id` | string | yes | Stable object identifier. |
| `source` | string | yes | L1 source family slug. |
| `domain` | string | yes | Source domain. |
| `object` | string | yes | Runtime source identity or logical object name. |
| `kind` | string | yes | Object kind such as `file`, `dir_listing`, `symlink`, `metadata`, `native_dump`, or `event_window`. |
| `status` | string | yes | Final object status from the object status vocabulary. |
| `path` | string or null | yes | Relative artifact path when content or metadata was written, otherwise null. |
| `bytes` | number or null | yes | Bytes written for file-like payloads, otherwise null. |
| `hash` | object or null | yes | Payload hash when computed, otherwise null. |
| `started_at_unix_ns` | string or null | yes | Task start timestamp when attempted. |
| `finished_at_unix_ns` | string or null | yes | Task finish timestamp when attempted. |
| `elapsed_us` | number or null | yes | Elapsed microseconds when attempted. |
| `limits` | object | yes | Effective task limits. |
| `reason` | string or null | yes | Human-readable reason for skipped, limited, unsupported, failed, or policy-sensitive outcomes. |

Baseline object kind values:

```text
file
file_set
dir_listing
symlink
metadata
metadata_only
bounded_tree
event_window
native_dump
```

Hashes are recommended for captured file-like payloads. Hashing should not delay
critical capture if the object is large and already protected by the snapshot
directory; planner policy may skip hashes for large objects.

## Plan

`plan.json` records planner decisions. It should explain:

- Host probes used for planning.
- Capability detection.
- Pressure and scale estimates.
- Task priority.
- Assigned budgets.
- Skip reasons.
- Conditional deepening triggers.

Schema compatibility rules for `plan.json` follow the same contract as
`manifest.json`: `schema_version` is a positive integer, readers must reject
unknown versions, readers should ignore unknown fields, and new optional fields
may be added without changing `schema_version`.

Top-level field contract:

| Field | Type | Required | Meaning |
|---|---:|---:|---|
| `schema_version` | number | yes | Plan schema version, independent from manifest schema version. |
| `planner` | string | yes | Planner strategy identifier, such as `adaptive-baseline`. |
| `probes` | object | yes | Host probes used for planning decisions. |
| `pressure` | object | yes | Detected runtime pressure levels. |
| `tasks` | array | yes | Planned task entries. |

Example:

```json
{
  "schema_version": 1,
  "planner": "adaptive-baseline",
  "probes": {
    "root": true,
    "cgroup_version": "v2",
    "systemd_detected": true,
    "container_runtime_sockets": ["containerd"]
  },
  "pressure": {
    "cpu": "low",
    "memory": "moderate",
    "io": "high"
  },
  "tasks": [
    {
      "id": "proc.system.stat",
      "source": "proc",
      "domain": "system",
      "object": "/proc/stat",
      "coverage_decision": "collect",
      "priority": "P0",
      "risk": "low",
      "decision": "scheduled",
      "reason": "core global runtime evidence",
      "limits": {
        "max_bytes": 1048576,
        "timeout_ms": 50,
        "max_files": 1,
        "max_depth": 0
      },
      "dependencies": []
    },
    {
      "id": "proc.process.smaps",
      "source": "proc",
      "domain": "process",
      "object": "/proc/<pid>/smaps",
      "coverage_decision": "limited",
      "priority": "P3",
      "risk": "high_cost",
      "decision": "limited",
      "reason": "io pressure is high; collect only selected suspicious PIDs",
      "limits": {
        "max_bytes": 1048576,
        "timeout_ms": 100,
        "max_files": 32,
        "max_depth": 0
      },
      "dependencies": ["proc.process.summary"]
    }
  ]
}
```

The plan does not need to expose every internal heuristic, but it must be useful
for explaining why a snapshot looks the way it does.

Required task fields:

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `id` | string | yes | Task identifier, usually matching the manifest object ID. |
| `source` | string | yes | L1 source family slug. |
| `domain` | string | yes | Source domain. |
| `object` | string | yes | Runtime source identity or logical object name. |
| `coverage_decision` | string | yes | Static decision from the Coverage Decision Matrix. |
| `decision` | string | yes | Runtime plan decision from the plan decision vocabulary. |
| `priority` | string | yes | `P0` through `P4`, or `NA` for non-scheduled decisions. |
| `risk` | string | yes | Planner risk level. |
| `reason` | string | yes | Explanation for the decision. |
| `limits` | object | yes | Effective limits if the task may run, otherwise the limits that prevented it. |
| `dependencies` | array | yes | Task IDs that influenced this task, empty when none. |

Plan decision vocabulary:

```text
scheduled
limited
skipped_by_policy
unsupported
not_present
```

Decision meanings:

- `scheduled`: the planner selected the task with normal limits for its coverage
  unit and priority.
- `limited`: the planner selected the task with stricter limits, selected-object
  policy, metadata-only policy, or reduced event window.
- `skipped_by_policy`: the planner intentionally did not run the task because of
  cost, pressure, risk, source overlap, release policy, or dependency failure.
- `unsupported`: the source is valid and may exist, but this implementation has no
  native collector for it.
- `not_present`: probing showed that the source or object was not present on this
  host.

## Decision and Status Mapping

Coverage decisions, plan decisions, and manifest statuses describe different
layers:

- Coverage decisions are static project intent.
- Plan decisions are per-host and per-capture scheduling decisions.
- Manifest statuses are observed final outcomes.

| Coverage decision | Typical plan decision | Typical manifest status |
|---|---|---|
| `collect` | `scheduled` | `captured`, `vanished`, `not_found`, `permission_denied`, `timeout`, `size_limited`, `truncated`, `io_error` |
| `conditional` | `scheduled`, `limited`, `skipped_by_policy`, `not_present`, `unsupported` | `captured`, `skipped_by_policy`, `not_found`, `unsupported`, or another attempted-task outcome |
| `limited` | `limited`, `skipped_by_policy`, `not_present`, `unsupported` | `captured`, `size_limited`, `truncated`, `skipped_by_policy`, `not_found`, `unsupported`, or another attempted-task outcome |
| `deferred-native` | `unsupported` | `unsupported` for important present sources; otherwise may appear only in `plan.json` |
| `exclude` | no task | no manifest object entry |

`limited` as a plan decision is not a failure. It means the task was deliberately
constrained before execution. `size_limited` and `truncated` are final object
statuses after execution or content-size evaluation.

## Errors

`errors.jsonl` is append-only. Each line is a JSON object:

```json
{"time_unix_ns":"1778328930123456789","task_id":"proc.123.fdinfo","status":"vanished","message":"process disappeared during fdinfo traversal","source":"proc","path":"/proc/123/fdinfo"}
```

Error lines should use the same status vocabulary as manifest object entries.
Sensitive raw data should not be added to error messages.

Each error line must include:

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `time_unix_ns` | string | yes | Event timestamp. |
| `task_id` | string or null | yes | Related task ID when known. |
| `status` | string | yes | Matching manifest status vocabulary. |
| `message` | string | yes | Short diagnostic message without raw sensitive payloads. |
| `source` | string or null | yes | Source family when known. |
| `path` | string or null | yes | Runtime path or logical object when safe to include. |

## Object Status Vocabulary

Baseline status values:

```text
captured
vanished
not_found
permission_denied
timeout
size_limited
truncated
skipped_by_policy
unsupported
io_error
```

Status meanings:

- `captured`: the object was captured within assigned limits.
- `vanished`: the object existed during discovery but disappeared before or
  during capture.
- `not_found`: the object was not present on this host.
- `permission_denied`: root was denied by kernel policy, LSM policy, or mount
  restrictions.
- `timeout`: the task exceeded its time budget.
- `size_limited`: the object was not captured as content because its estimated
  or observed size exceeded policy limits.
- `truncated`: partial content was intentionally written within policy limits.
- `skipped_by_policy`: the planner chose not to collect this object.
- `unsupported`: the source exists but no native collector is implemented.
- `io_error`: an unexpected I/O error occurred.

## Metadata Files

### meta/host.json

Host identity and stable runtime identity:

- Hostname.
- Boot ID.
- Kernel release and version.
- Machine architecture.
- Root filesystem device identity when available.
- Capture user and effective UID.

### meta/runtime.json

Capture runtime metadata:

- Tool version.
- Rust build metadata when available.
- Start and finish timestamps.
- Process ID of the capture process.
- Working directory.
- Output path.
- Whether `CAPTURE_COMPLETE` was written.

### meta/capabilities.json

Detected source availability:

- procfs, sysfs, cgroup, debugfs, tracefs, bpffs, securityfs, pstore.
- Netlink families.
- systemd or other service manager indicators.
- journald indicators.
- container daemon socket indicators.
- kernel features relevant to collection.

### meta/limits.json

Effective budgets:

- Global capture budget.
- Per-priority budgets.
- Per-source budgets.
- Default file size limit.
- Default traversal depth.
- Default directory entry limit.
- Default task timeout.

## Directory Permissions

Snapshot directories should be created with restrictive permissions because raw
runtime evidence may contain secrets:

```text
0700 for directories
0600 for regular files
```

Packaging should preserve restrictive permissions.

## Partial Snapshots

A partial snapshot is valid evidence. Offline tooling should report:

- Whether `CAPTURE_COMPLETE` exists.
- Which manifest files are present.
- Which source families started.
- Which object records are missing final status.
- The last error event.

The capture command should prefer leaving partial evidence over cleaning up a
failed snapshot directory.

Partial snapshot rules:

- Missing `CAPTURE_COMPLETE` means the snapshot is partial even if
  `manifest.json` exists.
- `complete: false` in `manifest.json` means the snapshot is partial even if a
  completion marker was created incorrectly.
- If `manifest.json` is missing but raw files exist, offline tooling should still
  preserve and expose the raw tree as unindexed evidence.
- Temporary files left by an interrupted capture are not authoritative manifest
  entries, but they may still be useful forensic artifacts.

## Archive Packaging

Archives are outside the critical capture path. A packaged snapshot must contain
the exact snapshot directory contents. Packaging must not rewrite raw evidence
or silently drop failed object records.

Archive metadata should include:

- Source snapshot directory.
- Packaging time.
- Compression format.
- Archive hash.
- Whether the snapshot was complete before packaging.
