# Safety and Operations

## Operating Assumption

`runfossil capture` is run by root during or immediately after a production
incident, before recovery actions such as restarting a service, killing a
process, deleting a container, or rebooting a host.

The tool must preserve evidence without making the incident worse.

## Root-Only Policy

Capture requires root. The command should fail early if the effective UID is not
zero.

Root-only operation is chosen because:

- Runtime evidence is often permission-restricted.
- Partial non-root captures are easy to misinterpret.
- Incident operators usually have elevated privileges during recovery.
- Planner decisions are simpler when root access is assumed.

Root does not mean unlimited access. Kernel lockdown, LSM policy, namespace
boundaries, mount options, and procfs restrictions may still deny access. Those
denials are expected manifest outcomes.

## Safe Rust Policy

All project-owned crates must forbid unsafe code:

```rust
#![forbid(unsafe_code)]
```

The implementation should prefer safe standard-library and safe crate APIs. If a
needed Linux interface has no safe Rust abstraction, the project should first
consider:

1. Whether the interface belongs in v1.
2. Whether a small audited safe crate already exists.
3. Whether the feature can be represented as unsupported until a safe design is
   available.

Unsafe code must not be introduced into this repository as an implementation
shortcut.

## External Command Policy

Collectors must not invoke external commands. This includes but is not limited
to:

```text
ip
ss
tc
lsof
ps
top
systemctl
loginctl
timedatectl
journalctl
docker
podman
ctr
crictl
runc
smartctl
nvme
ipmitool
nvidia-smi
```

Reasons:

- Commands vary by distribution and version.
- Commands can block, localize output, truncate output, or require config.
- Commands add hidden dependencies.
- Commands can have side effects or high startup cost.
- Native collection is easier to budget, test, and reason about.

## Non-Destructive Collection

Collectors must avoid operations that change runtime state. In particular:

- Do not write to `/proc/sys`, `/sys`, cgroup control files, debugfs, tracefs, or
  device control nodes.
- Do not enable tracing as part of capture.
- Do not clear kernel logs.
- Do not change service manager state.
- Do not pause, signal, ptrace, or otherwise control target processes.
- Do not open device nodes in modes that can block indefinitely or mutate state.
- Do not read infinite streams without strict bounds.

Capture should be read-only from the host's perspective except for writing the
snapshot output directory.

## Resource Limits

Every task must have limits:

- Timeout.
- Maximum bytes.
- Maximum files.
- Maximum recursion depth.
- Maximum child task expansion.
- Concurrency class.

The capture engine should have global limits:

- Maximum concurrent filesystem tasks.
- Maximum concurrent protocol tasks.
- Maximum outstanding writes.
- Maximum total runtime budget.
- Emergency stop threshold if the output filesystem becomes full or slow.

Limits are part of evidence quality. They must be recorded so analysts know why
an object is missing or truncated.

## Output Safety

The snapshot directory should be created with restrictive permissions:

```text
directories: 0700
files:       0600
```

The tool should avoid following untrusted symlink paths for writes. Runtime
symlink targets should be recorded as data, not followed as output locations.

The output path should be validated before capture:

- It must be writable.
- It should have enough free space for the planned capture.
- It should not be inside a source path that would cause self-capture recursion.
- It should not overwrite an existing completed snapshot.

## Sensitive Data

Snapshots may include:

- Command-line arguments.
- Environment variables.
- Usernames and UIDs.
- Network addresses and ports.
- File paths.
- Mount paths.
- Service metadata.
- Container metadata.
- Security and audit state.
- Kernel log messages.

The default product should preserve raw evidence rather than redact it. Redaction
can be added later as an offline operation. Operators must treat snapshot output
as sensitive.

## Failure Handling

Expected runtime races:

- Process exits during capture.
- Thread exits during capture.
- File descriptor closes during capture.
- Cgroup disappears during capture.
- Network socket closes during capture.
- Runtime files change between listing and read.
- Kernel buffers wrap during capture.

These are not fatal. They should be recorded as object outcomes.

Fatal conditions should be limited to:

- Not root.
- Cannot create snapshot directory.
- Cannot write required metadata.
- Output filesystem failure that prevents preserving evidence.
- Internal invariant violation.

Even after a fatal condition, the tool should preserve whatever evidence was
already written whenever possible.

## Performance Discipline

The capture path should prefer:

- Sequential small reads for critical files.
- Bounded concurrency for independent sources.
- Early persistence of core evidence.
- Avoiding full-tree traversal unless a tree is known and bounded.
- Avoiding large per-process reads under pressure.
- Avoiding compression until after capture.

The planner should reduce deep work when:

- PSI shows high I/O or memory pressure.
- The host has many processes or threads.
- Directory traversal is slow.
- Repeated timeouts occur.
- Output writes become slow.

## Testing Expectations

Testing should cover:

- Snapshot directory creation and partial snapshots.
- Manifest status vocabulary.
- Atomic write behavior.
- Path mapping and reserved names.
- Process disappearance races.
- Permission-denied outcomes.
- Budget enforcement.
- Planner determinism from fixture probes.
- No external command execution in collectors.
- No project-owned unsafe code.

Linux integration tests should use fixtures and temporary directories first.
Host-sensitive tests should be gated so they do not require a specific
distribution or kernel feature.

## Dependency Policy

Dependencies should be chosen conservatively:

- Prefer the standard library when reasonable.
- Prefer small crates with clear maintenance status.
- Prefer pure Rust crates.
- Avoid dependencies that primarily wrap command execution.
- Avoid dependencies that require C SDKs for core capture.
- Pin and review security-sensitive dependencies.
- Keep optional collectors behind optional features when they add meaningful
  dependency weight.

Because project-owned code forbids unsafe code, any dependency with internal
unsafe code must have a clear reason to exist.

## Operational Guidance

Recommended incident flow:

```text
1. Run `runfossil capture`.
2. Confirm the snapshot directory exists.
3. Proceed with recovery action.
4. Optionally run `runfossil pack <snapshot-dir>` after recovery.
5. Transfer the snapshot as a sensitive artifact for offline analysis.
```

Operators should not wait for packaging before restoring service. The raw
snapshot directory is the primary evidence artifact.
