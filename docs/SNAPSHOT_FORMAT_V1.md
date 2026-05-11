# Snapshot Format v1

## Goals

Snapshot Format v1 defines the on-disk evidence package produced by
`runfossil capture`.

The format must be:

- Useful when opened as a normal directory.
- Stable enough for offline tooling.
- Resilient to interrupted capture.
- Explicit about missing, skipped, truncated, and failed objects.
- Independent from compression or archive packaging.

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

The v1 layout is:

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
    container/
  CAPTURE_COMPLETE
```

`CAPTURE_COMPLETE` is written last. If it is missing, offline tooling must treat
the snapshot as partial.

## Atomicity Rules

- The snapshot directory is created before collection begins.
- Metadata and raw files are written through temporary files and then renamed
  into place.
- `errors.jsonl` may be append-only during capture.
- `manifest.json` and `plan.json` may be written initially as draft files, but
  their final versions must be atomically replaced.
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
container/containerd/containers.dump
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

Object entries should include:

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
  }
}
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

Example:

```json
{
  "schema_version": 1,
  "planner": "dynamic-v1",
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
      "priority": "P0",
      "risk": "low",
      "decision": "scheduled",
      "reason": "core global runtime evidence"
    },
    {
      "id": "proc.process.smaps",
      "priority": "P3",
      "risk": "high_cost",
      "decision": "limited",
      "reason": "io pressure is high; collect only selected suspicious PIDs"
    }
  ]
}
```

The plan does not need to expose every internal heuristic, but it must be useful
for explaining why a snapshot looks the way it does.

## Errors

`errors.jsonl` is append-only. Each line is a JSON object:

```json
{"time_unix_ns":"1778328930123456789","task_id":"proc.123.fdinfo","status":"vanished","message":"process disappeared during fdinfo traversal"}
```

Error lines should use the same status vocabulary as manifest object entries.
Sensitive raw data should not be added to error messages.

## Object Status Vocabulary

V1 status values:

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
- `size_limited`: the task was not read fully because it exceeded a size limit.
- `truncated`: partial content was intentionally written.
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
- container runtime sockets.
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
