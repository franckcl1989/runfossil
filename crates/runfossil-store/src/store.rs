#![forbid(unsafe_code)]
#![doc = "Snapshot store session: directory management, manifest tracking, error logging, and partial snapshot behavior."]

use std::path::{Path, PathBuf};

use runfossil_core::{ManifestStatus, SourceSlug};

use crate::manifest::ManifestEntry;
use crate::{
    COMPLETE_MARKER, ERRORS_FILE, MANIFEST_FILE, META_DIR, PLAN_FILE, RAW_DIR, SnapshotMetadata,
    StoreError, append_line, create_dir, create_dir_all, json_escape, write_atomic,
    write_atomic_bytes,
};

/// An append-only error log entry written to `errors.jsonl`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorLogEntry {
    /// Event timestamp in Unix nanoseconds as a decimal string.
    pub time_unix_ns: String,
    /// Related task ID when known.
    pub task_id: Option<String>,
    /// Matching manifest status vocabulary.
    pub status: ManifestStatus,
    /// Short diagnostic message without raw sensitive payloads.
    pub message: String,
    /// Source family when known.
    pub source: Option<SourceSlug>,
    /// Runtime path or logical object when safe to include.
    pub path: Option<String>,
}

impl ErrorLogEntry {
    /// Creates an error log entry.
    #[must_use]
    pub fn new(
        time_unix_ns: impl Into<String>,
        task_id: Option<impl Into<String>>,
        status: ManifestStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            time_unix_ns: time_unix_ns.into(),
            task_id: task_id.map(Into::into),
            status,
            message: message.into(),
            source: None,
            path: None,
        }
    }

    /// Sets the source family.
    #[must_use]
    pub fn with_source(mut self, source: SourceSlug) -> Self {
        self.source = Some(source);
        self
    }

    /// Sets the runtime path.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub(crate) fn to_json_line(&self) -> String {
        let task_id_json = match &self.task_id {
            Some(id) => format!("\"{}\"", json_escape(id)),
            None => "null".to_string(),
        };
        let source_json = match &self.source {
            Some(s) => format!("\"{}\"", json_escape(s.as_str())),
            None => "null".to_string(),
        };
        let path_json = match &self.path {
            Some(p) => format!("\"{}\"", json_escape(p)),
            None => "null".to_string(),
        };
        format!(
            "{{\"time_unix_ns\":\"{}\",\"task_id\":{},\"status\":\"{}\",\"message\":\"{}\",\"source\":{},\"path\":{}}}",
            json_escape(&self.time_unix_ns),
            task_id_json,
            json_escape(self.status.as_str()),
            json_escape(&self.message),
            source_json,
            path_json,
        )
    }
}

/// The live snapshot store session.
///
/// Created once per capture. Manages the snapshot directory, manifest entries,
/// error logging, and partial snapshot behavior. `CAPTURE_COMPLETE` is written
/// only on [`finalize`](Self::finalize). If the store is dropped without
/// finalization, the snapshot directory remains as partial evidence.
#[derive(Debug)]
pub struct SnapshotStore {
    snapshot_dir: PathBuf,
    metadata: SnapshotMetadata,
    manifest_entries: Vec<ManifestEntry>,
    finalized: bool,
}

impl SnapshotStore {
    /// Creates the snapshot directory, writes initial metadata files, and
    /// returns an open store session.
    ///
    /// The caller owns the store and must call [`finalize`](Self::finalize) to
    /// complete the snapshot. If the store is dropped without finalization, the
    /// directory remains as a partial snapshot.
    pub fn create(
        snapshot_dir: impl Into<PathBuf>,
        metadata: SnapshotMetadata,
    ) -> Result<Self, StoreError> {
        let snapshot_dir = snapshot_dir.into();

        create_dir(&snapshot_dir)?;
        create_dir(&snapshot_dir.join(META_DIR))?;
        create_dir(&snapshot_dir.join(RAW_DIR))?;

        for source in SourceSlug::all() {
            create_dir(&snapshot_dir.join(RAW_DIR).join(source.as_str()))?;
        }

        write_atomic(&snapshot_dir.join(ERRORS_FILE), "")?;
        write_atomic(
            &snapshot_dir.join(META_DIR).join("host.json"),
            &host_json(&metadata),
        )?;
        write_atomic(
            &snapshot_dir.join(META_DIR).join("runtime.json"),
            &initial_runtime_json(&metadata),
        )?;
        write_atomic(
            &snapshot_dir.join(META_DIR).join("capabilities.json"),
            &initial_capabilities_json(),
        )?;
        write_atomic(
            &snapshot_dir.join(META_DIR).join("limits.json"),
            &initial_limits_json(),
        )?;
        write_atomic(&snapshot_dir.join(PLAN_FILE), &initial_plan_json())?;

        Ok(Self {
            snapshot_dir,
            metadata,
            manifest_entries: Vec::new(),
            finalized: false,
        })
    }

    /// Returns the path to the snapshot directory.
    #[must_use]
    pub fn snapshot_dir(&self) -> &Path {
        &self.snapshot_dir
    }

    /// Records a manifest object entry in memory. Entries are written to
    /// `manifest.json` when [`finalize`](Self::finalize) is called.
    pub fn record_object(&mut self, entry: ManifestEntry) -> Result<(), StoreError> {
        if self.finalized {
            return Err(StoreError::AlreadyFinalized);
        }
        self.manifest_entries.push(entry);
        Ok(())
    }

    /// Appends an error log entry to `errors.jsonl`. Errors are written
    /// immediately (append-only) so that partial snapshots preserve error
    /// context even if the process is killed mid-capture.
    pub fn log_error(&self, entry: &ErrorLogEntry) -> Result<(), StoreError> {
        let path = self.snapshot_dir.join(ERRORS_FILE);
        append_line(&path, &entry.to_json_line())
    }

    /// Writes a raw evidence file atomically into the snapshot directory.
    ///
    /// The `artifact_path` must be relative to the snapshot root (e.g.
    /// `raw/proc/stat`). Parent directories are created as needed.
    pub fn write_raw_file(
        &self,
        artifact_path: impl AsRef<Path>,
        contents: &[u8],
    ) -> Result<(), StoreError> {
        let full_path = self.snapshot_dir.join(artifact_path.as_ref());
        if let Some(parent) = full_path.parent()
            && !parent.exists()
        {
            create_dir_all(parent)?;
        }
        write_atomic_bytes(&full_path, contents)
    }

    /// Finalizes the snapshot: writes the complete `manifest.json` with all
    /// recorded entries, updates `runtime.json` with the finish timestamp, and
    /// creates the `CAPTURE_COMPLETE` marker.
    ///
    /// After finalization, further writes are rejected.
    pub fn finalize(&mut self, finished_at_unix_ns: &str) -> Result<(), StoreError> {
        if self.finalized {
            return Err(StoreError::AlreadyFinalized);
        }

        let manifest_json = build_manifest_json(&self.metadata, &self.manifest_entries, true);
        write_atomic(&self.snapshot_dir.join(MANIFEST_FILE), &manifest_json)?;

        write_atomic(
            &self.snapshot_dir.join(META_DIR).join("runtime.json"),
            &final_runtime_json(&self.metadata, finished_at_unix_ns),
        )?;

        write_atomic(&self.snapshot_dir.join(COMPLETE_MARKER), "")?;

        self.finalized = true;
        Ok(())
    }

    /// Returns the number of manifest entries recorded so far.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.manifest_entries.len()
    }
}

fn host_json(metadata: &SnapshotMetadata) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"hostname\": \"{}\",\n",
            "  \"boot_id\": \"{}\",\n",
            "  \"kernel_release\": \"{}\",\n",
            "  \"machine\": \"{}\",\n",
            "  \"effective_uid\": {}\n",
            "}}\n"
        ),
        json_escape(&metadata.host.hostname),
        json_escape(&metadata.host.boot_id),
        json_escape(&metadata.host.kernel_release),
        json_escape(&metadata.host.machine),
        metadata.effective_uid.get()
    )
}

fn initial_runtime_json(metadata: &SnapshotMetadata) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"tool\": \"runfossil\",\n",
            "  \"tool_version\": \"{}\",\n",
            "  \"started_at_unix_ns\": \"{}\",\n",
            "  \"finished_at_unix_ns\": null,\n",
            "  \"complete\": false\n",
            "}}\n"
        ),
        json_escape(&metadata.tool_version),
        json_escape(&metadata.started_at_unix_ns)
    )
}

fn final_runtime_json(metadata: &SnapshotMetadata, finished_at_unix_ns: &str) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"tool\": \"runfossil\",\n",
            "  \"tool_version\": \"{}\",\n",
            "  \"started_at_unix_ns\": \"{}\",\n",
            "  \"finished_at_unix_ns\": \"{}\",\n",
            "  \"complete\": true\n",
            "}}\n"
        ),
        json_escape(&metadata.tool_version),
        json_escape(&metadata.started_at_unix_ns),
        json_escape(finished_at_unix_ns)
    )
}

fn initial_capabilities_json() -> String {
    concat!(
        "{\n",
        "  \"schema_version\": 1,\n",
        "  \"note\": \"milestone 2 skeleton; source probing is implemented in later milestones\"\n",
        "}\n"
    )
    .to_string()
}

fn initial_limits_json() -> String {
    concat!(
        "{\n",
        "  \"schema_version\": 1,\n",
        "  \"note\": \"milestone 2 skeleton; capture budgets are implemented in later milestones\"\n",
        "}\n"
    )
    .to_string()
}

fn initial_plan_json() -> String {
    concat!(
        "{\n",
        "  \"schema_version\": 1,\n",
        "  \"planner\": \"milestone-2-skeleton\",\n",
        "  \"probes\": {},\n",
        "  \"pressure\": {},\n",
        "  \"tasks\": []\n",
        "}\n"
    )
    .to_string()
}

fn build_manifest_json(
    metadata: &SnapshotMetadata,
    entries: &[ManifestEntry],
    complete: bool,
) -> String {
    let objects_json = entries
        .iter()
        .map(ManifestEntry::to_json)
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": 1,\n",
            "  \"tool\": \"runfossil\",\n",
            "  \"tool_version\": \"{}\",\n",
            "  \"started_at_unix_ns\": \"{}\",\n",
            "  \"finished_at_unix_ns\": \"{}\",\n",
            "  \"complete\": {},\n",
            "  \"root\": true,\n",
            "  \"host\": {{\n",
            "    \"hostname\": \"{}\",\n",
            "    \"boot_id\": \"{}\",\n",
            "    \"kernel_release\": \"{}\",\n",
            "    \"machine\": \"{}\"\n",
            "  }},\n",
            "  \"objects\": [\n{}\n  ]\n",
            "}}\n"
        ),
        json_escape(&metadata.tool_version),
        json_escape(&metadata.started_at_unix_ns),
        json_escape(&metadata.started_at_unix_ns),
        complete,
        json_escape(&metadata.host.hostname),
        json_escape(&metadata.host.boot_id),
        json_escape(&metadata.host.kernel_release),
        json_escape(&metadata.host.machine),
        objects_json,
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use runfossil_core::{EffectiveUid, ObjectKind};

    use super::*;
    use crate::HostMetadata;

    fn test_metadata() -> SnapshotMetadata {
        SnapshotMetadata {
            tool_version: "0.1.0".to_string(),
            started_at_unix_ns: "1".to_string(),
            effective_uid: EffectiveUid::ROOT,
            host: HostMetadata {
                hostname: "test-host".to_string(),
                boot_id: "boot-id".to_string(),
                kernel_release: "6.8.0".to_string(),
                machine: "x86_64".to_string(),
            },
        }
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_nanos(),
            Err(_) => 0,
        };
        std::env::temp_dir().join(format!(
            "runfossil-store-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn create_writes_initial_skeleton() -> Result<(), StoreError> {
        let dir = unique_test_dir("create-skeleton");
        let metadata = test_metadata();

        let store = SnapshotStore::create(&dir, metadata)?;

        assert!(dir.join(META_DIR).join("host.json").is_file());
        assert!(dir.join(META_DIR).join("runtime.json").is_file());
        assert!(dir.join(ERRORS_FILE).is_file());
        assert!(dir.join(PLAN_FILE).is_file());
        assert!(!dir.join(MANIFEST_FILE).is_file());
        assert!(!dir.join(COMPLETE_MARKER).is_file());

        drop(store);
        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }

    #[test]
    fn finalize_writes_manifest_and_completion_marker() -> Result<(), StoreError> {
        let dir = unique_test_dir("finalize");
        let metadata = test_metadata();

        let mut store = SnapshotStore::create(&dir, metadata)?;

        let entry = ManifestEntry::new(
            "proc.system.stat",
            SourceSlug::Proc,
            "system",
            "/proc/stat",
            ObjectKind::File,
            ManifestStatus::Captured,
        )
        .with_path("raw/proc/stat")
        .with_bytes(4096);

        store.record_object(entry)?;
        store.finalize("99")?;

        assert!(dir.join(MANIFEST_FILE).is_file());
        assert!(dir.join(COMPLETE_MARKER).is_file());

        let manifest_content = fs::read_to_string(dir.join(MANIFEST_FILE))
            .map_err(|e| StoreError::io("read manifest", e))?;
        assert!(manifest_content.contains("\"id\": \"proc.system.stat\""));
        assert!(manifest_content.contains("\"status\": \"captured\""));
        assert!(manifest_content.contains("\"complete\": true"));

        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }

    #[test]
    fn errors_append_to_jsonl() -> Result<(), StoreError> {
        let dir = unique_test_dir("errors");
        let metadata = test_metadata();

        let store = SnapshotStore::create(&dir, metadata)?;

        let e1 = ErrorLogEntry::new(
            "100",
            Some("task.1"),
            ManifestStatus::Vanished,
            "process exited during capture",
        )
        .with_source(SourceSlug::Proc)
        .with_path("/proc/42/status");

        let e2 = ErrorLogEntry::new(
            "200",
            None::<&str>,
            ManifestStatus::PermissionDenied,
            "cannot read file",
        );

        store.log_error(&e1)?;
        store.log_error(&e2)?;

        let errors_content = fs::read_to_string(dir.join(ERRORS_FILE))
            .map_err(|e| StoreError::io("read errors", e))?;

        assert!(errors_content.contains("\"status\":\"vanished\""));
        assert!(errors_content.contains("\"status\":\"permission_denied\""));
        assert!(errors_content.contains("\"task_id\":\"task.1\""));

        drop(store);
        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }

    #[test]
    fn partial_snapshot_has_no_completion_marker() -> Result<(), StoreError> {
        let dir = unique_test_dir("partial");
        let metadata = test_metadata();

        let mut store = SnapshotStore::create(&dir, metadata)?;

        let entry = ManifestEntry::new(
            "proc.system.uptime",
            SourceSlug::Proc,
            "system",
            "/proc/uptime",
            ObjectKind::File,
            ManifestStatus::Captured,
        );
        store.record_object(entry)?;

        let error = ErrorLogEntry::new(
            "50",
            Some("proc.system.uptime"),
            ManifestStatus::IoError,
            "simulated failure before finalize",
        );
        store.log_error(&error)?;

        drop(store);

        assert!(dir.is_dir());
        assert!(dir.join(META_DIR).is_dir());
        assert!(dir.join(ERRORS_FILE).is_file());
        assert!(!dir.join(COMPLETE_MARKER).is_file());

        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }

    #[test]
    fn finalize_twice_rejects() -> Result<(), StoreError> {
        let dir = unique_test_dir("finalize-twice");
        let metadata = test_metadata();

        let mut store = SnapshotStore::create(&dir, metadata)?;
        store.finalize("10")?;
        let result = store.finalize("20");
        assert!(result.is_err());

        drop(store);
        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }

    #[test]
    fn record_after_finalize_rejects() -> Result<(), StoreError> {
        let dir = unique_test_dir("record-after-finalize");
        let metadata = test_metadata();

        let mut store = SnapshotStore::create(&dir, metadata)?;
        store.finalize("10")?;

        let entry = ManifestEntry::new(
            "late.entry",
            SourceSlug::Proc,
            "system",
            "/late",
            ObjectKind::File,
            ManifestStatus::Captured,
        );
        let result = store.record_object(entry);
        assert!(matches!(result, Err(StoreError::AlreadyFinalized)));

        drop(store);
        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }

    #[test]
    fn multiple_manifest_entries_preserved() -> Result<(), StoreError> {
        let dir = unique_test_dir("multiple-entries");
        let metadata = test_metadata();

        let mut store = SnapshotStore::create(&dir, metadata)?;

        for i in 0..5 {
            store.record_object(
                ManifestEntry::new(
                    format!("proc.process.{i}.status"),
                    SourceSlug::Proc,
                    "process",
                    format!("/proc/{i}/status"),
                    ObjectKind::File,
                    ManifestStatus::Captured,
                )
                .with_bytes(512),
            )?;
        }

        assert_eq!(store.entry_count(), 5);
        store.finalize("999")?;

        let manifest = fs::read_to_string(dir.join(MANIFEST_FILE))
            .map_err(|e| StoreError::io("read manifest", e))?;
        assert!(manifest.contains("\"id\": \"proc.process.0.status\""));
        assert!(manifest.contains("\"id\": \"proc.process.4.status\""));

        drop(store);
        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }

    #[test]
    fn error_log_entry_json_matches_spec() {
        let entry = ErrorLogEntry::new(
            "1778328930123456789",
            Some("proc.123.fdinfo"),
            ManifestStatus::Vanished,
            "process disappeared during fdinfo traversal",
        )
        .with_source(SourceSlug::Proc)
        .with_path("/proc/123/fdinfo");

        let line = entry.to_json_line();
        assert!(line.contains("\"time_unix_ns\":\"1778328930123456789\""));
        assert!(line.contains("\"task_id\":\"proc.123.fdinfo\""));
        assert!(line.contains("\"status\":\"vanished\""));
        assert!(line.contains("\"source\":\"proc\""));
        assert!(line.contains("\"path\":\"/proc/123/fdinfo\""));
    }

    #[test]
    fn write_raw_file_creates_parent_dirs() -> Result<(), StoreError> {
        let dir = unique_test_dir("raw-file");
        let metadata = test_metadata();

        let store = SnapshotStore::create(&dir, metadata)?;

        store.write_raw_file("raw/proc/stat", b"cpu  0 0 0\n")?;

        let file_path = dir.join("raw/proc/stat");
        assert!(file_path.is_file());
        let contents = fs::read_to_string(&file_path).map_err(|e| StoreError::io("read", e))?;
        assert_eq!(contents, "cpu  0 0 0\n");

        drop(store);
        fs::remove_dir_all(&dir).map_err(|e| StoreError::io("cleanup", e))?;
        Ok(())
    }
}
