#![forbid(unsafe_code)]
#![doc = "Service manager, session, time, and log collectors for runfossil."]

mod dbus;
mod logind;
mod logs;
mod systemd;
mod time_sync;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

/// Returns the source slug owned by this collector crate.
#[must_use]
pub const fn source() -> SourceSlug {
    SourceSlug::Service
}

/// Collects service manager, session, time, and event-log state.
pub fn collect_service(store: &mut SnapshotStore) -> Result<(), StoreError> {
    record_service_detection(store)?;
    time_sync::collect_time_state(store)?;
    logs::collect_log_windows(store)?;

    let bus = match dbus::connect_system_bus() {
        Ok(stream) => Some(stream),
        Err(error) => {
            record_dbus_unavailable(store, &error)?;
            None
        }
    };

    if let Some(mut stream) = bus {
        systemd::collect_systemd_state(store, &mut stream)?;
        logind::collect_logind_state(store, &mut stream)?;
    }

    Ok(())
}

fn record_service_detection(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let detected = systemd::detect_systemd();
    let content = format!(
        "service_manager: systemd\nsystemd_detected: {}\n",
        if detected { "true" } else { "false" }
    );

    let out = std::path::Path::new("raw/service/service_detection");
    store.write_raw_file(out, content.as_bytes())?;

    let entry = ManifestEntry::new(
        "service.detection",
        SourceSlug::Service,
        "service",
        "detection",
        ObjectKind::Metadata,
        ManifestStatus::Captured,
    )
    .with_path(out.to_string_lossy())
    .with_bytes(content.len() as u64)
    .with_limits(ObjectLimits::new(256, 50, 1, 0));

    store.record_object(entry)?;
    Ok(())
}

fn record_dbus_unavailable(
    store: &mut SnapshotStore,
    error: &std::io::Error,
) -> Result<(), StoreError> {
    let content = format!("dbus_system_bus: unavailable\nerror: {error}\n");
    let out = std::path::Path::new("raw/service/dbus_status");
    store.write_raw_file(out, content.as_bytes())?;

    let entry = ManifestEntry::new(
        "service.dbus.status",
        SourceSlug::Service,
        "dbus",
        "status",
        ObjectKind::Metadata,
        ManifestStatus::NotFound,
    )
    .with_reason(format!("D-Bus system bus unavailable: {error}"))
    .with_path(out.to_string_lossy())
    .with_bytes(content.len() as u64)
    .with_limits(ObjectLimits::new(256, 50, 1, 0));

    store.record_object(entry)?;
    Ok(())
}
