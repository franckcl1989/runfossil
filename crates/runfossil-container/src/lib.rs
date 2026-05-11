#![forbid(unsafe_code)]
#![doc = "Container runtime collector for runfossil."]

use std::path::Path;

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ManifestEntry, SnapshotStore, StoreError};

mod cgroup;
mod docker_api;
mod namespace;
mod runtime;

/// Returns the source slug owned by this collector crate.
#[must_use]
pub const fn source() -> SourceSlug {
    SourceSlug::Container
}

/// Collects container runtime state into the given snapshot store.
///
/// The collector runs independently: it records detection metadata first,
/// then collects /run container state, host-side cgroup and namespace evidence,
/// and Docker Engine API dumps when the socket is available.
pub fn collect_container(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let detected = record_detection(store)?;

    if !detected {
        return Ok(());
    }

    runtime::collect_run_state(store)?;
    docker_api::collect_docker_state(store)?;

    let container_pids = namespace::discover_container_pids();
    namespace::collect_namespace_evidence(store, &container_pids)?;

    cgroup::collect_cgroup_evidence(store)?;

    Ok(())
}

fn record_detection(store: &mut SnapshotStore) -> Result<bool, StoreError> {
    let docker = Path::new("/run/docker.sock").exists();
    let containerd = Path::new("/run/containerd/containerd.sock").exists();
    let crio = Path::new("/run/crio/crio.sock").exists();
    let runc = Path::new("/run/runc").is_dir();
    let docker_run = Path::new("/run/docker").is_dir();
    let containerd_run = Path::new("/run/containerd").is_dir();
    let crio_run = Path::new("/run/crio").is_dir();

    let detected = docker || containerd || crio || runc || docker_run || containerd_run || crio_run;

    let status = if detected {
        ManifestStatus::Captured
    } else {
        ManifestStatus::NotFound
    };

    let details = format!(
        "docker_sock={docker},containerd_sock={containerd},crio_sock={crio},\
         runc_dir={runc},docker_rundir={docker_run},containerd_rundir={containerd_run},\
         crio_rundir={crio_run}"
    );

    let entry = ManifestEntry::new(
        "container.detection",
        SourceSlug::Container,
        "detection",
        "detection",
        ObjectKind::Metadata,
        status,
    )
    .with_reason(details);

    store.record_object(entry)?;
    Ok(detected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runfossil_core::EffectiveUid;
    use runfossil_store::{HostMetadata, SnapshotMetadata};

    fn test_metadata() -> SnapshotMetadata {
        SnapshotMetadata {
            tool_version: "0.1.0".into(),
            started_at_unix_ns: "1".into(),
            effective_uid: EffectiveUid::ROOT,
            host: HostMetadata {
                hostname: "test".into(),
                boot_id: "test-boot".into(),
                kernel_release: "6.8.0".into(),
                machine: "x86_64".into(),
            },
        }
    }

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn new_store(d: &tempfile::TempDir) -> SnapshotStore {
        SnapshotStore::create(d.path().join("snapshot"), test_metadata()).unwrap()
    }

    #[test]
    fn source_returns_container_slug() {
        assert_eq!(source(), SourceSlug::Container);
    }

    #[test]
    fn collect_container_with_no_runtimes_records_not_found() {
        let d = dir();
        let mut store = new_store(&d);
        let result = collect_container(&mut store);
        assert!(result.is_ok());
    }

    #[test]
    fn cgroup_collector_returns_ok_when_fs_absent() {
        let d = dir();
        let mut store = new_store(&d);
        let result = cgroup::collect_cgroup_evidence(&mut store);
        assert!(result.is_ok());
    }

    #[test]
    fn runtime_collector_returns_ok_when_dirs_absent() {
        let d = dir();
        let mut store = new_store(&d);
        let result = runtime::collect_run_state(&mut store);
        assert!(result.is_ok());
    }

    #[test]
    fn namespace_evidence_with_empty_pids_returns_ok() {
        let d = dir();
        let mut store = new_store(&d);
        let result = namespace::collect_namespace_evidence(&mut store, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn namespace_evidence_with_nonexistent_pids_returns_ok() {
        let d = dir();
        let mut store = new_store(&d);
        let result = namespace::collect_namespace_evidence(&mut store, &[99999, 99998]);
        assert!(result.is_ok());
    }

    #[test]
    fn is_container_related_matches_docker_markers() {
        assert!(cgroup::is_container_related("docker-abc123.scope"));
        assert!(cgroup::is_container_related("containerd-def456.scope"));
        assert!(cgroup::is_container_related("crio-ghi789.scope"));
        assert!(cgroup::is_container_related("kubepods-besteffort.slice"));
        assert!(cgroup::is_container_related("libpod-jkl012.scope"));
    }

    #[test]
    fn is_container_related_rejects_non_container() {
        assert!(!cgroup::is_container_related("system.slice"));
        assert!(!cgroup::is_container_related("user.slice"));
        assert!(!cgroup::is_container_related("user-1000.slice"));
        assert!(!cgroup::is_container_related("sshd.service"));
    }

    #[test]
    fn is_container_related_empty_string() {
        assert!(!cgroup::is_container_related(""));
    }
}
