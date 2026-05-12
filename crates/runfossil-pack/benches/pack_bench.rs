#![forbid(unsafe_code)]
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]
use std::fs;
use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use runfossil_core::{EffectiveUid, ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{HostMetadata, ManifestEntry, ObjectLimits, SnapshotMetadata, SnapshotStore};
use std::hint::black_box;

fn create_test_snapshot(base: &Path) {
    let snapshot_dir = base.join("snapshot");
    let metadata = SnapshotMetadata {
        tool_version: "0.1.0".into(),
        started_at_unix_ns: "1".into(),
        effective_uid: EffectiveUid::ROOT,
        host: HostMetadata {
            hostname: "bench".into(),
            boot_id: "bench-boot".into(),
            kernel_release: "6.8.0".into(),
            machine: "x86_64".into(),
        },
    };

    let mut store = SnapshotStore::create(&snapshot_dir, metadata).unwrap();

    for i in 0..100u32 {
        let content = format!("bench file {i:04}\n").repeat(50);
        let path = format!("raw/data/file_{i}.dat");
        store
            .write_raw_file(Path::new(&path), content.as_bytes())
            .unwrap();

        let entry = ManifestEntry::new(
            format!("data.{i}"),
            SourceSlug::Proc,
            "bench",
            format!("{i}"),
            ObjectKind::File,
            ManifestStatus::Captured,
        )
        .with_path(path)
        .with_bytes(content.len() as u64)
        .with_limits(ObjectLimits::new(65536, 200, 1, 0));

        store.record_object(entry).unwrap();
    }

    store.finalize("2").unwrap();
}

fn bench_pack_small(c: &mut Criterion) {
    let temp = tempfile::tempdir().unwrap();
    create_test_snapshot(temp.path());

    c.bench_function("pack_100_files", |b| {
        b.iter(|| {
            let result =
                runfossil_pack::pack_snapshot(black_box(temp.path().join("snapshot").as_path()));
            let _ = result.unwrap();
        })
    });

    let archive = temp
        .path()
        .parent()
        .unwrap_or(temp.path())
        .join("snapshot.tar.zst");
    let _ = fs::remove_file(&archive);
}

fn bench_pack_large(c: &mut Criterion) {
    let temp = tempfile::tempdir().unwrap();
    let snapshot_dir = temp.path().join("snapshot");
    let metadata = SnapshotMetadata {
        tool_version: "0.1.0".into(),
        started_at_unix_ns: "1".into(),
        effective_uid: EffectiveUid::ROOT,
        host: HostMetadata {
            hostname: "bench".into(),
            boot_id: "bench-boot".into(),
            kernel_release: "6.8.0".into(),
            machine: "x86_64".into(),
        },
    };

    {
        let mut store = SnapshotStore::create(&snapshot_dir, metadata).unwrap();
        for i in 0..500u32 {
            let content = format!("large benchmark file {i:05}\n").repeat(100);
            let path = format!("raw/large/file_{i}.dat");
            store
                .write_raw_file(Path::new(&path), content.as_bytes())
                .unwrap();
            let entry = ManifestEntry::new(
                format!("large.{i}"),
                SourceSlug::Proc,
                "bench",
                format!("{i}"),
                ObjectKind::File,
                ManifestStatus::Captured,
            )
            .with_path(path)
            .with_bytes(content.len() as u64)
            .with_limits(ObjectLimits::new(65536, 200, 1, 0));
            store.record_object(entry).unwrap();
        }
        store.finalize("2").unwrap();
    }

    c.bench_function("pack_500_files", |b| {
        b.iter(|| {
            let result = runfossil_pack::pack_snapshot(black_box(snapshot_dir.as_path()));
            let _ = result.unwrap();
        })
    });
}

fn bench_inspect(c: &mut Criterion) {
    let temp = tempfile::tempdir().unwrap();
    create_test_snapshot(temp.path());

    c.bench_function("inspect_100_files", |b| {
        b.iter(|| {
            let result =
                runfossil_pack::inspect_snapshot(black_box(temp.path().join("snapshot").as_path()));
            let _ = result.unwrap();
        })
    });
}

criterion_group!(benches, bench_pack_small, bench_pack_large, bench_inspect);
criterion_main!(benches);
