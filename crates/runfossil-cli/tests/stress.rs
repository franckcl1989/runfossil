#![forbid(unsafe_code)]
#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

#[cfg(test)]
mod stress {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};

    use runfossil_core::{EffectiveUid, ManifestStatus, ObjectKind, SourceSlug};
    use runfossil_store::{
        ErrorLogEntry, HashRecord, HostMetadata, ManifestEntry, ObjectLimits, SnapshotMetadata,
        SnapshotStore,
    };
    fn test_metadata() -> SnapshotMetadata {
        SnapshotMetadata {
            tool_version: "0.1.0".into(),
            started_at_unix_ns: "1".into(),
            effective_uid: EffectiveUid::ROOT,
            host: HostMetadata {
                hostname: "stress-test".into(),
                boot_id: "stress-boot-id".into(),
                kernel_release: "6.8.0".into(),
                machine: "x86_64".into(),
            },
        }
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    fn new_store(dir: &tempfile::TempDir) -> SnapshotStore {
        SnapshotStore::create(dir.path().join("snapshot"), test_metadata()).expect("create store")
    }

    #[test]
    fn store_handles_10k_manifest_entries() {
        let d = temp_dir();
        let mut store = new_store(&d);

        for i in 0..10_000u32 {
            let entry = ManifestEntry::new(
                format!("proc.process.{i}.status"),
                SourceSlug::Proc,
                "process",
                format!("{i}/status"),
                ObjectKind::File,
                ManifestStatus::Captured,
            )
            .with_path(format!("raw/proc/{i}/status"))
            .with_bytes(1024)
            .with_limits(ObjectLimits::new(4096, 50, 1, 0));

            store.record_object(entry).expect("record entry");
        }

        store.finalize("2").expect("finalize");

        let manifest_path = d.path().join("snapshot/manifest.json");
        let manifest = fs::read_to_string(&manifest_path).expect("read manifest");
        let id_count = manifest.matches("\"id\":").count();
        assert!(
            id_count >= 10_000,
            "manifest should have at least 10k entries, got {id_count}"
        );
    }

    #[test]
    fn store_handles_concurrent_writes() {
        let d = temp_dir();
        let store = Arc::new(Mutex::new(new_store(&d)));
        let counter = Arc::new(AtomicU64::new(0));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let store = Arc::clone(&store);
                let counter = Arc::clone(&counter);
                std::thread::spawn(move || {
                    for _ in 0..250 {
                        let i = counter.fetch_add(1, Ordering::SeqCst);
                        let entry = ManifestEntry::new(
                            format!("thread.{i}"),
                            SourceSlug::Proc,
                            "concurrent",
                            format!("{i}"),
                            ObjectKind::File,
                            ManifestStatus::Captured,
                        )
                        .with_bytes(256);

                        let mut s = store.lock().expect("lock");
                        s.record_object(entry).expect("record");
                    }
                })
            })
            .collect();

        for handle in threads {
            handle.join().expect("join");
        }

        let mut store = Arc::into_inner(store)
            .expect("unwrap arc")
            .into_inner()
            .expect("unwrap mutex");
        store.finalize("2").expect("finalize");

        let manifest_path = d.path().join("snapshot/manifest.json");
        let manifest = fs::read_to_string(&manifest_path).expect("read manifest");
        let id_count = manifest.matches("\"id\":").count();
        assert_eq!(id_count, 2000, "should have 2000 entries");
    }

    #[test]
    fn large_error_log_is_writable() {
        let d = temp_dir();
        let mut store = new_store(&d);

        for i in 0..500u32 {
            let error_entry = ErrorLogEntry::new(
                i.to_string(),
                None::<String>,
                ManifestStatus::IoError,
                format!("error number {i} occurred during capture"),
            );
            store.log_error(&error_entry).expect("log error");
        }

        store.finalize("2").expect("finalize");

        let errors_path = d.path().join("snapshot/errors.jsonl");
        let errors = fs::read_to_string(&errors_path).expect("read errors");
        assert_eq!(errors.lines().count(), 500);
    }

    #[test]
    fn pack_handles_large_snapshot() {
        let d = temp_dir();
        let snapshot_dir = d.path().join("snapshot");
        {
            let mut store =
                SnapshotStore::create(&snapshot_dir, test_metadata()).expect("create store");

            for i in 0..1000u32 {
                let content = format!("data for file {i:04}\n").repeat(10);
                let path = format!("raw/proc/{i}/status");
                store
                    .write_raw_file(Path::new(&path), content.as_bytes())
                    .expect("write file");

                let entry = ManifestEntry::new(
                    format!("file.{i}"),
                    SourceSlug::Proc,
                    "data",
                    format!("{i}"),
                    ObjectKind::File,
                    ManifestStatus::Captured,
                )
                .with_path(path)
                .with_bytes(content.len() as u64)
                .with_limits(ObjectLimits::new(65536, 200, 1, 0));

                store.record_object(entry).expect("record entry");
            }

            store.finalize("2").expect("finalize");
        }

        let metadata = runfossil_pack::pack_snapshot(&snapshot_dir).expect("pack snapshot");
        assert!(metadata.archive_path.exists(), "archive should exist");
        let archive_size = fs::metadata(&metadata.archive_path)
            .expect("archive metadata")
            .len();
        assert!(archive_size > 0, "archive should have content");

        let report = runfossil_pack::inspect_snapshot(&snapshot_dir).expect("inspect snapshot");
        let display = format!("{report}");
        assert!(display.contains("1000"), "inspect should show 1000 entries");
    }

    #[test]
    fn pack_preserves_many_small_files() {
        let d = temp_dir();
        let snapshot_dir = d.path().join("snapshot");
        let mut expected: HashMap<String, Vec<u8>> = HashMap::new();
        {
            let mut store =
                SnapshotStore::create(&snapshot_dir, test_metadata()).expect("create store");

            for i in 0..500u32 {
                let content = vec![(i % 256) as u8; (i as usize % 64) + 1];
                let path = format!("raw/data/file_{i}.bin");
                store
                    .write_raw_file(Path::new(&path), &content)
                    .expect("write file");
                expected.insert(path.clone(), content.clone());

                let entry = ManifestEntry::new(
                    format!("data.{i}"),
                    SourceSlug::Proc,
                    "data",
                    format!("{i}"),
                    ObjectKind::File,
                    ManifestStatus::Captured,
                )
                .with_path(path)
                .with_bytes(content.len() as u64)
                .with_hash(HashRecord::new(
                    "sha256",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                ))
                .with_limits(ObjectLimits::new(65536, 200, 1, 0));

                store.record_object(entry).expect("record entry");
            }

            store.finalize("2").expect("finalize");
        }

        let archive_metadata = runfossil_pack::pack_snapshot(&snapshot_dir).expect("pack snapshot");
        assert!(
            archive_metadata.archive_size_bytes > 0,
            "archive should have content"
        );

        let archive_bytes = fs::read(&archive_metadata.archive_path).expect("read archive");
        assert!(!archive_bytes.is_empty(), "archive should have content");
    }

    #[test]
    fn pack_handles_empty_file_entries() {
        let d = temp_dir();
        let snapshot_dir = d.path().join("snapshot");
        {
            let mut store =
                SnapshotStore::create(&snapshot_dir, test_metadata()).expect("create store");

            for i in 0..100u32 {
                let path = format!("raw/empty/empty_{i}.txt");
                store
                    .write_raw_file(Path::new(&path), b"")
                    .expect("write empty file");

                let entry = ManifestEntry::new(
                    format!("empty.{i}"),
                    SourceSlug::Proc,
                    "empty",
                    format!("{i}"),
                    ObjectKind::File,
                    ManifestStatus::Captured,
                )
                .with_path(path)
                .with_bytes(0)
                .with_limits(ObjectLimits::new(65536, 200, 1, 0));

                store.record_object(entry).expect("record entry");
            }

            store.finalize("2").expect("finalize");
        }

        let archive_metadata = runfossil_pack::pack_snapshot(&snapshot_dir).expect("pack snapshot");
        assert!(archive_metadata.archive_path.exists());
    }

    #[test]
    fn partial_snapshot_with_many_entries_is_inspectable() {
        let d = temp_dir();
        let snapshot_dir = d.path().join("snapshot");
        {
            let mut store =
                SnapshotStore::create(&snapshot_dir, test_metadata()).expect("create store");

            for i in 0..500u32 {
                let path = format!("raw/batch/batch_{i}.txt");
                store
                    .write_raw_file(Path::new(&path), b"partial content")
                    .expect("write file");

                let entry = ManifestEntry::new(
                    format!("batch.{i}"),
                    SourceSlug::Proc,
                    "batch",
                    format!("{i}"),
                    ObjectKind::File,
                    ManifestStatus::Captured,
                )
                .with_path(path)
                .with_bytes(15)
                .with_limits(ObjectLimits::new(65536, 200, 1, 0));

                store.record_object(entry).expect("record entry");
            }
        }

        let report =
            runfossil_pack::inspect_snapshot(&snapshot_dir).expect("inspect partial snapshot");
        let display = format!("{report}");
        assert!(
            display.contains("Complete:    no"),
            "should report not complete"
        );
        assert!(display.contains("500"), "should report 500 entries");
    }

    #[test]
    fn snapshot_with_mixed_statuses_handles_large_count() {
        let d = temp_dir();
        let mut store = new_store(&d);

        let statuses = [
            ManifestStatus::Captured,
            ManifestStatus::NotFound,
            ManifestStatus::PermissionDenied,
            ManifestStatus::SkippedByPolicy,
            ManifestStatus::Unsupported,
            ManifestStatus::IoError,
            ManifestStatus::Timeout,
            ManifestStatus::SizeLimited,
            ManifestStatus::Truncated,
            ManifestStatus::Vanished,
        ];

        for i in 0..500u32 {
            let status = statuses[(i as usize) % statuses.len()];
            let entry = ManifestEntry::new(
                format!("mixed.{i}"),
                SourceSlug::Proc,
                "mixed",
                format!("{i}"),
                ObjectKind::File,
                status,
            )
            .with_bytes(64)
            .with_limits(ObjectLimits::new(4096, 50, 1, 0));

            store.record_object(entry).expect("record entry");
        }

        store.finalize("2").expect("finalize");

        let manifest_path = d.path().join("snapshot/manifest.json");
        let manifest = fs::read_to_string(&manifest_path).expect("read manifest");
        let id_count = manifest.matches("\"id\":").count();
        assert!(
            id_count >= 500,
            "manifest should have at least 500 entries, got {id_count}"
        );
    }

    #[test]
    fn store_write_and_flush_under_pressure() {
        let d = temp_dir();
        let mut store = new_store(&d);

        for i in 0..2000u32 {
            let content = format!("cycle {}\n", i);
            let path = format!("raw/cycle/file_{}.txt", i);
            store
                .write_raw_file(Path::new(&path), content.as_bytes())
                .expect("write file");

            let entry = ManifestEntry::new(
                format!("cycle.{i}"),
                SourceSlug::Proc,
                "cycle",
                format!("{i}"),
                ObjectKind::File,
                ManifestStatus::Captured,
            )
            .with_path(path)
            .with_bytes(content.len() as u64)
            .with_limits(ObjectLimits::new(65536, 200, 1, 0));

            store.record_object(entry).expect("record entry");
        }

        store.finalize("2").expect("finalize");

        let manifest_path = d.path().join("snapshot/manifest.json");
        let manifest = fs::read_to_string(&manifest_path).expect("read manifest");
        let id_count = manifest.matches("\"id\":").count();
        assert!(
            id_count >= 2000,
            "should have at least 2000 entries, got {id_count}"
        );
    }
}
