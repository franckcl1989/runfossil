#![doc = "systemd D-Bus collector: manager state and unit list via native protocol."]

use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ErrorLogEntry, ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

use crate::dbus;

const MAX_RESPONSE_BYTES: u64 = 8_388_608;

/// Detects whether systemd is the active service manager.
#[must_use]
pub(crate) fn detect_systemd() -> bool {
    Path::new("/run/systemd").is_dir()
        && (Path::new("/run/dbus/system_bus_socket").exists()
            || Path::new("/var/run/dbus/system_bus_socket").exists())
}

/// Collects systemd manager state via D-Bus ListUnits.
pub(crate) fn collect_systemd_state(
    store: &mut SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    collect_list_units(store, stream)?;
    Ok(())
}

fn collect_list_units(
    store: &mut SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    let dest = "org.freedesktop.systemd1";
    let path = "/org/freedesktop/systemd1";
    let iface = "org.freedesktop.systemd1.Manager";
    let out_path = "raw/service/systemd/ListUnits.dump";

    match dbus::dbus_call(stream, dest, path, iface, "ListUnits", &[]) {
        Ok(body) => {
            let bytes = body.len() as u64;
            store.write_raw_file(Path::new(out_path), &body)?;

            let entry = ManifestEntry::new(
                "service.systemd.ListUnits.dump",
                SourceSlug::Service,
                "systemd",
                "ListUnits",
                ObjectKind::NativeDump,
                ManifestStatus::Captured,
            )
            .with_path(out_path)
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 5000, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                "service.systemd.ListUnits.dump",
                SourceSlug::Service,
                "systemd",
                "ListUnits",
                ObjectKind::NativeDump,
                ManifestStatus::IoError,
            )
            .with_reason(format!("systemd ListUnits D-Bus call failed: {error}"))
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 5000, 1, 0));

            store.record_object(entry)?;
            let error_log = ErrorLogEntry::new(
                unix_time_ns_string(),
                None::<String>,
                ManifestStatus::IoError,
                format!("systemd ListUnits D-Bus call failed: {error}"),
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
