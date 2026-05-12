#![forbid(unsafe_code)]
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use runfossil_core::{EffectiveUid, ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{HostMetadata, ManifestEntry, ObjectLimits, SnapshotMetadata, SnapshotStore};

fn metadata() -> SnapshotMetadata {
    SnapshotMetadata {
        tool_version: "0.1.0".into(),
        started_at_unix_ns: "1".into(),
        effective_uid: EffectiveUid::ROOT,
        host: HostMetadata {
            hostname: "bench".into(),
            boot_id: "bench-boot".into(),
            kernel_release: "6.8.0".into(),
            machine: "x86_64".into(),
        },
    }
}

fn bench_store_create(c: &mut Criterion) {
    c.bench_function("store_create", |b| {
        b.iter(|| {
            let temp = tempfile::tempdir().unwrap();
            let snapshot_dir = temp.path().join("snapshot");
            let meta = metadata();
            let result = SnapshotStore::create(black_box(&snapshot_dir), black_box(meta));
            let _ = result.unwrap();
        })
    });
}

fn bench_manifest_entry_creation(c: &mut Criterion) {
    c.bench_function("manifest_entry_create", |b| {
        b.iter(|| {
            let _ = ManifestEntry::new(
                black_box("bench.object.1"),
                black_box(SourceSlug::Proc),
                black_box("bench"),
                black_box("1"),
                black_box(ObjectKind::File),
                black_box(ManifestStatus::Captured),
            )
            .with_path(black_box("raw/bench/1"))
            .with_bytes(black_box(1024))
            .with_limits(black_box(ObjectLimits::new(4096, 50, 1, 0)));
        })
    });
}

fn bench_store_record_and_finalize(c: &mut Criterion) {
    c.bench_function("store_record_100_finalize", |b| {
        b.iter(|| {
            let temp = tempfile::tempdir().unwrap();
            let snapshot_dir = temp.path().join("snapshot");
            let meta = metadata();
            let mut store = SnapshotStore::create(&snapshot_dir, meta).unwrap();

            for i in 0..100u32 {
                let content = format!("bench data {i}\n").repeat(10);
                let path = format!("raw/bench/file_{i}");
                store
                    .write_raw_file(Path::new(&path), content.as_bytes())
                    .unwrap();
                let entry = ManifestEntry::new(
                    format!("bench.{i}"),
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
        })
    });
}

criterion_group!(
    benches,
    bench_store_create,
    bench_manifest_entry_creation,
    bench_store_record_and_finalize
);
criterion_main!(benches);
