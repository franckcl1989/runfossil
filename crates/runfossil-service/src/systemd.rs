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
    collect_manager_state(store, stream)?;
    collect_unit_details(store, stream)?;
    collect_failed_state(store, stream)?;
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

fn collect_manager_state(
    store: &mut SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    let dest = "org.freedesktop.systemd1";
    let path = "/org/freedesktop/systemd1";
    let props_iface = "org.freedesktop.DBus.Properties";
    let mgr_iface = "org.freedesktop.systemd1.Manager";
    let out_path = "raw/service/systemd/manager.json";

    let properties = [
        "Version",
        "Features",
        "Architecture",
        "Virtualization",
        "ControlGroup",
    ];
    let mut values: Vec<(&str, String)> = Vec::new();

    for prop in &properties {
        match dbus::dbus_call(stream, dest, path, props_iface, "Get", &[mgr_iface, prop]) {
            Ok(body) => {
                if let Some(val) = dbus::take_variant_string(&body) {
                    values.push((prop, val.0.to_string()));
                } else {
                    values.push((prop, String::new()));
                }
            }
            Err(error) => {
                let error_log = ErrorLogEntry::new(
                    unix_time_ns_string(),
                    None::<String>,
                    ManifestStatus::IoError,
                    format!("systemd Properties.Get({prop}) failed: {error}"),
                );
                let _ = store.log_error(&error_log);
            }
        }
    }

    let json = build_simple_json_object(&values);
    let bytes = json.len() as u64;
    store.write_raw_file(Path::new(out_path), json.as_bytes())?;

    let entry = ManifestEntry::new(
        "service.manager.state",
        SourceSlug::Service,
        "manager",
        "state",
        ObjectKind::NativeDump,
        ManifestStatus::Captured,
    )
    .with_path(out_path)
    .with_bytes(bytes)
    .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 5000, 1, 0));

    store.record_object(entry)?;
    Ok(())
}

fn collect_unit_details(
    store: &mut SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    let dest = "org.freedesktop.systemd1";
    let path = "/org/freedesktop/systemd1";
    let iface = "org.freedesktop.systemd1.Manager";
    let out_path = "raw/service/systemd/units.json";

    let units_body = match dbus::dbus_call(stream, dest, path, iface, "ListUnits", &[]) {
        Ok(body) => body,
        Err(error) => {
            let entry = ManifestEntry::new(
                "service.systemd.units",
                SourceSlug::Service,
                "systemd",
                "units",
                ObjectKind::NativeDump,
                ManifestStatus::IoError,
            )
            .with_reason(format!("systemd units D-Bus call failed: {error}"))
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 5000, 1, 0));

            store.record_object(entry)?;
            let error_log = ErrorLogEntry::new(
                unix_time_ns_string(),
                None::<String>,
                ManifestStatus::IoError,
                format!("systemd ListUnits for units parsing failed: {error}"),
            );
            store.log_error(&error_log)?;
            return Ok(());
        }
    };

    let names_with_slots = dbus::extract_list_units_names(&units_body, 32);
    let mut mapping: Vec<(&str, String)> = Vec::new();

    for (name, _) in &names_with_slots {
        match dbus::dbus_call(stream, dest, path, iface, "GetUnit", &[name]) {
            Ok(body) => {
                if let Some(object_path) = dbus::parse_object_path(&body) {
                    mapping.push((name, object_path.to_string()));
                } else {
                    mapping.push((name, String::new()));
                }
            }
            Err(error) => {
                let error_log = ErrorLogEntry::new(
                    unix_time_ns_string(),
                    None::<String>,
                    ManifestStatus::IoError,
                    format!("systemd GetUnit({name}) failed: {error}"),
                );
                let _ = store.log_error(&error_log);
            }
        }
    }

    let json = build_simple_json_object(&mapping);
    let bytes = json.len() as u64;
    store.write_raw_file(Path::new(out_path), json.as_bytes())?;

    let entry = ManifestEntry::new(
        "service.systemd.units",
        SourceSlug::Service,
        "systemd",
        "units",
        ObjectKind::NativeDump,
        ManifestStatus::Captured,
    )
    .with_path(out_path)
    .with_bytes(bytes)
    .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 5000, 1, 0));

    store.record_object(entry)?;
    Ok(())
}

fn collect_failed_state(
    store: &mut SnapshotStore,
    stream: &mut UnixStream,
) -> Result<(), StoreError> {
    let dest = "org.freedesktop.systemd1";
    let path = "/org/freedesktop/systemd1";
    let iface = "org.freedesktop.systemd1.Manager";
    let out_path = "raw/service/systemd/failed.json";

    match dbus::dbus_call(stream, dest, path, iface, "ListUnitsFiltered", &["failed"]) {
        Ok(body) => {
            let bytes = body.len() as u64;
            store.write_raw_file(Path::new(out_path), &body)?;

            let entry = ManifestEntry::new(
                "service.systemd.failed",
                SourceSlug::Service,
                "units",
                "failed",
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
                "service.systemd.failed",
                SourceSlug::Service,
                "units",
                "failed",
                ObjectKind::NativeDump,
                ManifestStatus::IoError,
            )
            .with_reason(format!("systemd ListUnitsFiltered failed: {error}"))
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, 5000, 1, 0));

            store.record_object(entry)?;
            let error_log = ErrorLogEntry::new(
                unix_time_ns_string(),
                None::<String>,
                ManifestStatus::IoError,
                format!("systemd ListUnitsFiltered failed: {error}"),
            );
            store.log_error(&error_log)?;
        }
    }

    Ok(())
}

fn build_simple_json_object(entries: &[(&str, String)]) -> String {
    let mut parts = Vec::new();
    for (key, value) in entries {
        let escaped_key = key.replace('\\', "\\\\").replace('\"', "\\\"");
        let escaped_val = value.replace('\\', "\\\\").replace('\"', "\\\"");
        parts.push(format!("\"{escaped_key}\": \"{escaped_val}\""));
    }
    format!("{{{}}}", parts.join(", "))
}

fn unix_time_ns_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
