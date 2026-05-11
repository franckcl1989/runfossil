#![doc = "logind session state collector via D-Bus native protocol."]

use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ErrorLogEntry, ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::dbus;

const MAX_RESPONSE_BYTES: u64 = 2_097_152;

/// Collects logind session state via D-Bus ListSessions.
pub(crate) fn collect_logind_state(
    store: &mut SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    let dest = "org.freedesktop.login1";
    let path = "/org/freedesktop/login1";
    let iface = "org.freedesktop.login1.Manager";
    let out_path = "raw/service/logind/ListSessions.dump";

    match dbus::dbus_call(stream, dest, path, iface, "ListSessions", &[]) {
        Ok(body) => {
            let bytes = body.len() as u64;
            store.write_raw_file(Path::new(out_path), &body)?;

            let entry = ManifestEntry::new(
                "sessions.logind.ListSessions.dump",
                SourceSlug::Sessions,
                "logind",
                "ListSessions",
                ObjectKind::NativeDump,
                ManifestStatus::Captured,
            )
            .with_path(out_path)
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 3000, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                "sessions.logind.ListSessions.dump",
                SourceSlug::Sessions,
                "logind",
                "ListSessions",
                ObjectKind::NativeDump,
                ManifestStatus::IoError,
            )
            .with_reason(format!("logind ListSessions D-Bus call failed: {error}"))
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 3000, 1, 0));

            store.record_object(entry)?;
            let error_log = ErrorLogEntry::new(
                unix_time_ns_string(),
                None::<String>,
                ManifestStatus::IoError,
                format!("logind ListSessions D-Bus call failed: {error}"),
            );
            store.log_error(&error_log)?;
        }
    }

    Ok(())
}

fn unix_time_ns_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
