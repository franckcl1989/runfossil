#![forbid(unsafe_code)]
#![doc = "Time synchronization and clock state collector."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_fs::{BoundedReadLimits, read_file_bounded};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

/// Collects system clock and time synchronization state.
pub(crate) fn collect_time_state(store: &SnapshotStore) -> Result<(), StoreError> {
    collect_uptime(store)?;
    record_btime(store)?;
    collect_localtime(store)?;
    collect_timesync_status(store)?;
    Ok(())
}

fn collect_uptime(store: &SnapshotStore) -> Result<(), StoreError> {
    let limits = BoundedReadLimits::new(256, 50);
    let source = Path::new("/proc/uptime");

    match read_file_bounded(source, limits) {
        Ok(result) => {
            let out_path = Path::new("raw/time/uptime");
            let bytes = result.content.len() as u64;
            store.write_raw_file(out_path, &result.content)?;

            let entry = ManifestEntry::new(
                "time.uptime",
                SourceSlug::Time,
                "clock",
                "uptime",
                ObjectKind::File,
                ManifestStatus::Captured,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(limits.max_bytes, limits.timeout_ms, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                "time.uptime",
                SourceSlug::Time,
                "clock",
                "uptime",
                ObjectKind::File,
                ManifestStatus::IoError,
            )
            .with_reason(format!("/proc/uptime: {error}"))
            .with_limits(ObjectLimits::new(256, 50, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn record_btime(store: &SnapshotStore) -> Result<(), StoreError> {
    let limits = BoundedReadLimits::new(4_096, 50);
    let source = Path::new("/proc/stat");

    match read_file_bounded(source, limits) {
        Ok(result) => {
            let out_path = Path::new("raw/time/btime");
            let content = String::from_utf8_lossy(&result.content);
            let mut btime = String::new();

            for line in content.lines() {
                if line.starts_with("btime ") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        btime = format!("btime: {value}\n");
                    }
                    break;
                }
            }

            if btime.is_empty() {
                btime = "btime: not_found\n".to_string();
            }

            let bytes = btime.len() as u64;
            store.write_raw_file(out_path, btime.as_bytes())?;

            let entry = ManifestEntry::new(
                "time.btime",
                SourceSlug::Time,
                "clock",
                "btime",
                ObjectKind::File,
                ManifestStatus::Captured,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(128, 50, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                "time.btime",
                SourceSlug::Time,
                "clock",
                "btime",
                ObjectKind::File,
                ManifestStatus::IoError,
            )
            .with_reason(format!("/proc/stat: {error}"))
            .with_limits(ObjectLimits::new(128, 50, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn collect_localtime(store: &SnapshotStore) -> Result<(), StoreError> {
    let loc = Path::new("/etc/localtime");

    if !loc.exists() {
        let entry = ManifestEntry::new(
            "time.localtime",
            SourceSlug::Time,
            "clock",
            "localtime",
            ObjectKind::Symlink,
            ManifestStatus::NotFound,
        )
        .with_reason("/etc/localtime not present")
        .with_limits(ObjectLimits::new(0, 50, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    match std::fs::read_link(loc) {
        Ok(target) => {
            let out_path = Path::new("raw/time/localtime");
            let content = target.to_string_lossy().into_owned();
            store.write_raw_file(out_path, content.as_bytes())?;

            let entry = ManifestEntry::new(
                "time.localtime",
                SourceSlug::Time,
                "clock",
                "localtime",
                ObjectKind::Symlink,
                ManifestStatus::Captured,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(content.len() as u64)
            .with_limits(ObjectLimits::new(256, 50, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                "time.localtime",
                SourceSlug::Time,
                "clock",
                "localtime",
                ObjectKind::Symlink,
                ManifestStatus::IoError,
            )
            .with_reason(format!("/etc/localtime: {error}"))
            .with_limits(ObjectLimits::new(0, 50, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn collect_timesync_status(store: &SnapshotStore) -> Result<(), StoreError> {
    let timesync_dir = Path::new("/run/systemd/timesync");

    if !timesync_dir.exists() {
        let entry = ManifestEntry::new(
            "time.timesyncd.status",
            SourceSlug::Time,
            "timesyncd",
            "status",
            ObjectKind::Metadata,
            ManifestStatus::NotFound,
        )
        .with_reason("systemd-timesyncd not detected on this host")
        .with_limits(ObjectLimits::new(0, 50, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let limits = BoundedReadLimits::new(4_096, 100);
    let files = ["clock", "frequency", "root.drift"];

    for name in &files {
        let source_path = timesync_dir.join(name);
        match read_file_bounded(&source_path, limits) {
            Ok(result) => {
                let out_path = Path::new("raw/time").join(format!("timesyncd_{name}"));
                let bytes = result.content.len() as u64;
                store.write_raw_file(&out_path, &result.content)?;

                let entry = ManifestEntry::new(
                    format!("time.timesyncd.{name}"),
                    SourceSlug::Time,
                    "timesyncd",
                    *name,
                    ObjectKind::File,
                    ManifestStatus::Captured,
                )
                .with_path(out_path.to_string_lossy())
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(
                    limits.max_bytes,
                    limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(entry)?;
            }
            Err(error) => {
                let entry = ManifestEntry::new(
                    format!("time.timesyncd.{name}"),
                    SourceSlug::Time,
                    "timesyncd",
                    *name,
                    ObjectKind::File,
                    ManifestStatus::IoError,
                )
                .with_reason(error.to_string())
                .with_limits(ObjectLimits::new(
                    limits.max_bytes,
                    limits.timeout_ms,
                    1,
                    0,
                ));

                store.record_object(entry)?;
            }
        }
    }

    Ok(())
}
