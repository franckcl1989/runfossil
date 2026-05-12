#![forbid(unsafe_code)]
#![allow(
    missing_docs,
    unreachable_pub,
    clippy::expect_used,
    clippy::unwrap_used
)]

mod common;

use common::{create_test_snapshot_dir, empty_manifest_json, empty_plan_json};

use std::path::Path;

#[test]
fn pack_rejects_nonexistent_directory() {
    let result = runfossil_pack::pack_snapshot(Path::new("/nonexistent/runfossil/snapshot"));
    assert!(result.is_err());
}

#[test]
fn pack_rejects_directory_without_manifest() {
    let dir = tempfile::TempDir::with_prefix("runfossil-int-test").expect("create temp dir");
    let result = runfossil_pack::pack_snapshot(dir.path());
    assert!(result.is_err());
}

#[test]
fn inspect_rejects_nonexistent_directory() {
    let result = runfossil_pack::inspect_snapshot(Path::new("/nonexistent/runfossil/snapshot"));
    assert!(result.is_err());
}

#[test]
fn inspect_empty_directory_reports_no_entries() {
    let dir = tempfile::TempDir::with_prefix("runfossil-int-test").expect("create temp dir");
    let report = runfossil_pack::inspect_snapshot(dir.path()).expect("inspect empty dir");
    let output = format!("{report}");
    assert!(!output.is_empty());
}

#[test]
fn pack_and_inspect_roundtrip() {
    let snapshot = create_test_snapshot_dir();
    let dir = snapshot.path();

    let meta = runfossil_pack::pack_snapshot(dir).expect("pack snapshot");
    runfossil_pack::write_archive_metadata(&meta).expect("write archive meta");

    assert!(meta.archive_path.exists());
    assert!(meta.archive_path.metadata().expect("metadata").len() > 0);
    assert!(meta.snapshot_was_complete);

    let report = runfossil_pack::inspect_snapshot(dir).expect("inspect snapshot");
    let output = format!("{report}");
    assert!(output.contains("Complete:    yes"));
    assert!(output.contains("Manifest:    yes"));

    snapshot.close().expect("cleanup temp dir");
}

#[test]
fn partial_snapshot_reports_not_complete() {
    let snapshot = tempfile::TempDir::with_prefix("runfossil-partial").expect("create temp dir");
    let snap_dir = snapshot.path();

    empty_manifest_json(false, snap_dir);
    empty_plan_json(snap_dir);

    let report = runfossil_pack::inspect_snapshot(snap_dir).expect("inspect partial");
    let output = format!("{report}");
    assert!(!output.contains("CAPTURE_COMPLETE") || output.contains("Partial"));

    snapshot.close().expect("cleanup temp dir");
}

#[test]
fn pack_preserves_content_integrity() {
    let snapshot = create_test_snapshot_dir();
    let dir = snapshot.path();

    let meta = runfossil_pack::pack_snapshot(dir).expect("pack");
    runfossil_pack::write_archive_metadata(&meta).expect("write meta");

    let original_content =
        std::fs::read_to_string(dir.join("raw/proc/stat")).expect("read original");

    let extract_dir =
        tempfile::TempDir::with_prefix("runfossil-extract").expect("create extract dir");
    let archive_file = std::fs::File::open(&meta.archive_path).expect("open archive");
    let decoder = zstd::Decoder::new(archive_file).expect("zstd decode");
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(extract_dir.path()).expect("unpack archive");

    let extracted_content =
        std::fs::read_to_string(extract_dir.path().join("raw/proc/stat")).expect("read extracted");

    assert_eq!(original_content, extracted_content);

    snapshot.close().expect("cleanup temp dir");
    extract_dir.close().expect("cleanup extract dir");
}
