#![forbid(unsafe_code)]
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
    store: &SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    collect_list_sessions(store, stream)?;
    collect_session_details(store, stream)?;
    collect_user_sessions(store, stream)?;
    Ok(())
}

fn collect_list_sessions(store: &SnapshotStore, stream: &mut UnixStream) -> Result<(), StoreError> {
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

fn collect_session_details(
    store: &SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    let dest = "org.freedesktop.login1";
    let path = "/org/freedesktop/login1";
    let iface = "org.freedesktop.login1.Manager";

    let sessions_body = match dbus::dbus_call(stream, dest, path, iface, "ListSessions", &[]) {
        Ok(body) => body,
        Err(error) => {
            let error_log = ErrorLogEntry::new(
                unix_time_ns_string(),
                None::<String>,
                ManifestStatus::IoError,
                format!("logind session details ListSessions failed: {error}"),
            );
            store.log_error(&error_log)?;
            return Ok(());
        }
    };

    let ids_with_slots = dbus::extract_list_sessions_ids(&sessions_body, 32);

    for (id, _) in &ids_with_slots {
        let out_path = format!("raw/service/logind/session_{id}.json");
        match dbus::dbus_call(stream, dest, path, iface, "GetSession", &[id]) {
            Ok(body) => {
                let bytes = body.len() as u64;
                store.write_raw_file(Path::new(&out_path), &body)?;

                let entry = ManifestEntry::new(
                    format!("sessions.logind.session_{id}"),
                    SourceSlug::Sessions,
                    "logind",
                    format!("session_{id}"),
                    ObjectKind::NativeDump,
                    ManifestStatus::Captured,
                )
                .with_path(&out_path)
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 3000, 1, 0));

                store.record_object(entry)?;
            }
            Err(error) => {
                let error_log = ErrorLogEntry::new(
                    unix_time_ns_string(),
                    None::<String>,
                    ManifestStatus::IoError,
                    format!("logind GetSession({id}) failed: {error}"),
                );
                let _ = store.log_error(&error_log);
            }
        }
    }

    Ok(())
}

fn collect_user_sessions(store: &SnapshotStore, stream: &mut UnixStream) -> Result<(), StoreError> {
    let dest = "org.freedesktop.login1";
    let path = "/org/freedesktop/login1";
    let iface = "org.freedesktop.login1.Manager";
    let out_path = "raw/service/logind/users.json";

    match dbus::dbus_call(stream, dest, path, iface, "ListUsers", &[]) {
        Ok(body) => {
            let bytes = body.len() as u64;
            store.write_raw_file(Path::new(out_path), &body)?;

            let entry = ManifestEntry::new(
                "sessions.logind.ListUsers.dump",
                SourceSlug::Sessions,
                "users",
                "users",
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
                "sessions.logind.ListUsers.dump",
                SourceSlug::Sessions,
                "users",
                "users",
                ObjectKind::NativeDump,
                ManifestStatus::IoError,
            )
            .with_reason(format!("logind ListUsers D-Bus call failed: {error}"))
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 3000, 1, 0));

            store.record_object(entry)?;
            let error_log = ErrorLogEntry::new(
                unix_time_ns_string(),
                None::<String>,
                ManifestStatus::IoError,
                format!("logind ListUsers D-Bus call failed: {error}"),
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
