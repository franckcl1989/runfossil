#![forbid(unsafe_code)]
#![doc = "Kernel ring buffer bounded window collector. Reads /dev/kmsg with byte and time bounds."]

use std::fs;
use std::io::Read;
use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::now_ns;

const KMSG_MAX_BYTES: u64 = 2_097_152;
const KMSG_TIMEOUT_MS: u64 = 500;

/// Collects a bounded window from the kernel ring buffer via `/dev/kmsg`.
///
/// Reads up to `KMSG_MAX_BYTES` from `/dev/kmsg`. The content is preserved
/// as raw binary and written to `raw/kernel/kmsg.window`. If `/dev/kmsg` is
/// not present (e.g., kernel not configured with CONFIG_PRINTK), a
/// `not_found` entry is recorded.
pub(crate) fn collect_kmsg(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let kmsg_path = Path::new("/dev/kmsg");
    if !kmsg_path.exists() {
        let entry = ManifestEntry::new(
            "kernel.kmsg.window",
            SourceSlug::Kernel,
            "kmsg",
            "window",
            ObjectKind::EventWindow,
            ManifestStatus::NotFound,
        )
        .with_reason("/dev/kmsg not present")
        .with_limits(ObjectLimits::new(KMSG_MAX_BYTES, KMSG_TIMEOUT_MS, 1, 0));

        store.record_object(entry)?;
        return Ok(());
    }

    let started = now_ns();

    match read_kmsg_window(KMSG_MAX_BYTES) {
        Ok(content) => {
            let out_path = Path::new("raw/kernel/kmsg.window");
            let finished = now_ns();
            let bytes = content.len() as u64;

            store.write_raw_file(out_path, &content)?;

            let entry = ManifestEntry::new(
                "kernel.kmsg.window",
                SourceSlug::Kernel,
                "kmsg",
                "window",
                ObjectKind::EventWindow,
                ManifestStatus::Captured,
            )
            .with_path(out_path.to_string_lossy())
            .with_bytes(bytes)
            .with_timing(started, finished, 0)
            .with_limits(ObjectLimits::new(KMSG_MAX_BYTES, KMSG_TIMEOUT_MS, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let finished = now_ns();
            let entry = ManifestEntry::new(
                "kernel.kmsg.window",
                SourceSlug::Kernel,
                "kmsg",
                "window",
                ObjectKind::EventWindow,
                ManifestStatus::IoError,
            )
            .with_reason(error.to_string())
            .with_limits(ObjectLimits::new(KMSG_MAX_BYTES, KMSG_TIMEOUT_MS, 1, 0));

            store.record_object(entry)?;

            store.log_error(
                &runfossil_store::ErrorLogEntry::new(
                    finished,
                    None::<String>,
                    ManifestStatus::IoError,
                    format!("failed to read /dev/kmsg: {error}"),
                )
                .with_source(SourceSlug::Kernel)
                .with_path("/dev/kmsg"),
            )?;
        }
    }

    Ok(())
}

fn read_kmsg_window(max_bytes: u64) -> Result<Vec<u8>, std::io::Error> {
    let mut file = fs::File::open("/dev/kmsg")?;
    let mut content = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut total_read: u64 = 0;

    loop {
        let remaining = max_bytes.saturating_sub(total_read);
        if remaining == 0 {
            break;
        }

        let chunk_size = (remaining as usize).min(buffer.len());
        match file.read(&mut buffer[..chunk_size]) {
            Ok(0) => break,
            Ok(bytes_read) => {
                total_read += bytes_read as u64;
                content.extend_from_slice(&buffer[..bytes_read]);
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    break;
                }
                return Err(e);
            }
        }
    }

    Ok(content)
}
