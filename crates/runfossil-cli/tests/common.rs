#![forbid(unsafe_code)]
#![allow(missing_docs, unreachable_pub, dead_code)]

use std::fs;
use std::path::Path;

pub fn empty_manifest_json(complete: bool, snapshot_dir: &Path) {
    let manifest = format!(
        r#"{{
  "schema_version": 1,
  "tool": "runfossil",
  "tool_version": "0.1.0",
  "started_at_unix_ns": "0",
  "finished_at_unix_ns": "0",
  "complete": {},
  "root": false,
  "host": {{
    "hostname": "test",
    "boot_id": "00000000-0000-0000-0000-000000000000",
    "kernel_release": "6.8.0",
    "machine": "x86_64"
  }},
  "objects": []
}}"#,
        complete
    );
    fs::create_dir_all(snapshot_dir).expect("create snapshot dir");
    fs::write(snapshot_dir.join("manifest.json"), manifest).expect("write manifest");
}

pub fn empty_plan_json(snapshot_dir: &Path) {
    let plan = r#"{
  "schema_version": 1,
  "planner": "adaptive-baseline",
  "probes": {
    "root": false,
    "cgroup_version": "none",
    "systemd_detected": false,
    "container_runtime_sockets": []
  },
  "pressure": {
    "cpu": "low",
    "memory": "low",
    "io": "low"
  },
  "tasks": []
}"#;
    fs::create_dir_all(snapshot_dir).expect("create snapshot dir");
    fs::write(snapshot_dir.join("plan.json"), plan).expect("write plan");
}

pub fn create_test_snapshot_dir() -> tempfile::TempDir {
    let dir = tempfile::TempDir::with_prefix("runfossil-itest-snapshot").expect("create temp dir");
    let snap = dir.path();

    // Create directory structure
    fs::create_dir_all(snap.join("raw/proc")).expect("create raw/proc");
    fs::create_dir_all(snap.join("raw/sys")).expect("create raw/sys");
    fs::create_dir_all(snap.join("raw/netlink")).expect("create raw/netlink");
    fs::create_dir_all(snap.join("meta")).expect("create meta");

    // Write manifest with completion marker
    empty_manifest_json(true, snap);
    empty_plan_json(snap);

    // Write sample raw files
    fs::write(snap.join("raw/proc/stat"), "cpu  100 0 50 200 10\n").expect("write proc/stat");
    fs::write(snap.join("raw/proc/loadavg"), "0.10 0.20 0.15 1/100 1234\n").expect("write loadavg");
    fs::write(snap.join("raw/netlink/link.dump"), b"\x00\x01\x02\x03").expect("write link.dump");

    // Write completion marker
    fs::write(snap.join("CAPTURE_COMPLETE"), "").expect("write CAPTURE_COMPLETE");

    // Write errors
    fs::write(snap.join("errors.jsonl"), "").expect("write errors.jsonl");

    // Write meta files
    fs::write(
        snap.join("meta/host.json"),
        r#"{"hostname":"test","boot_id":"00000000-0000-0000-0000-000000000000","kernel_release":"6.8.0","machine":"x86_64"}"#,
    )
    .expect("write host.json");

    fs::write(
        snap.join("meta/runtime.json"),
        r#"{"tool_version":"0.1.0","started_at_unix_ns":"0","finished_at_unix_ns":"0","complete":true}"#,
    )
    .expect("write runtime.json");

    dir
}
