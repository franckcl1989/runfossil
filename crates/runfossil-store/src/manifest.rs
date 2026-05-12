#![forbid(unsafe_code)]
#![doc = "Manifest entry types and serialization for runfossil snapshot metadata."]

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};

use crate::{json_escape, json_optional_str, json_optional_u64};

/// A hash record for a captured object payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashRecord {
    /// Hash algorithm name.
    pub algorithm: String,
    /// Hash value as a hex string.
    pub value: String,
}

impl HashRecord {
    /// Creates a hash record.
    #[must_use]
    pub fn new(algorithm: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            algorithm: algorithm.into(),
            value: value.into(),
        }
    }

    pub(crate) fn to_json(&self) -> String {
        format!(
            "{{\"algorithm\":\"{}\",\"value\":\"{}\"}}",
            json_escape(&self.algorithm),
            json_escape(&self.value)
        )
    }
}

/// Effective limits applied during object capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectLimits {
    /// Maximum bytes for file-like payloads.
    pub max_bytes: u64,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum files for traversal tasks.
    pub max_files: u64,
    /// Maximum traversal depth.
    pub max_depth: u32,
}

impl ObjectLimits {
    /// Creates object limits.
    #[must_use]
    pub const fn new(max_bytes: u64, timeout_ms: u64, max_files: u64, max_depth: u32) -> Self {
        Self {
            max_bytes,
            timeout_ms,
            max_files,
            max_depth,
        }
    }

    /// Returns default object limits for small file-like evidence.
    #[must_use]
    pub const fn default_for_small_file() -> Self {
        Self {
            max_bytes: 1_048_576,
            timeout_ms: 50,
            max_files: 1,
            max_depth: 0,
        }
    }

    pub(crate) fn to_json(self) -> String {
        format!(
            "{{\"max_bytes\":{},\"timeout_ms\":{},\"max_files\":{},\"max_depth\":{}}}",
            self.max_bytes, self.timeout_ms, self.max_files, self.max_depth
        )
    }
}

/// A manifest object entry representing one capture outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestEntry {
    /// Stable object identifier.
    pub id: String,
    /// L1 source family slug.
    pub source: SourceSlug,
    /// L2 source domain.
    pub domain: String,
    /// Runtime source identity or logical object name.
    pub object: String,
    /// Object kind.
    pub kind: ObjectKind,
    /// Final object status.
    pub status: ManifestStatus,
    /// Relative artifact path when content was written.
    pub path: Option<String>,
    /// Bytes written for file-like payloads.
    pub bytes: Option<u64>,
    /// Payload hash when computed.
    pub hash: Option<HashRecord>,
    /// Task start timestamp in Unix nanoseconds.
    pub started_at_unix_ns: Option<String>,
    /// Task finish timestamp in Unix nanoseconds.
    pub finished_at_unix_ns: Option<String>,
    /// Elapsed microseconds.
    pub elapsed_us: Option<u64>,
    /// Effective task limits.
    pub limits: ObjectLimits,
    /// Human-readable reason for the outcome.
    pub reason: Option<String>,
}

impl ManifestEntry {
    /// Creates a manifest entry with minimal required fields.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        source: SourceSlug,
        domain: impl Into<String>,
        object: impl Into<String>,
        kind: ObjectKind,
        status: ManifestStatus,
    ) -> Self {
        Self {
            id: id.into(),
            source,
            domain: domain.into(),
            object: object.into(),
            kind,
            status,
            path: None,
            bytes: None,
            hash: None,
            started_at_unix_ns: None,
            finished_at_unix_ns: None,
            elapsed_us: None,
            limits: ObjectLimits::default_for_small_file(),
            reason: None,
        }
    }

    /// Sets the artifact path.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Sets the byte count.
    #[must_use]
    pub const fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes = Some(bytes);
        self
    }

    /// Sets the hash record.
    #[must_use]
    pub fn with_hash(mut self, hash: HashRecord) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Sets timing fields.
    #[must_use]
    pub fn with_timing(
        mut self,
        started_at_unix_ns: impl Into<String>,
        finished_at_unix_ns: impl Into<String>,
        elapsed_us: u64,
    ) -> Self {
        self.started_at_unix_ns = Some(started_at_unix_ns.into());
        self.finished_at_unix_ns = Some(finished_at_unix_ns.into());
        self.elapsed_us = Some(elapsed_us);
        self
    }

    /// Sets the reason.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Sets the limits.
    #[must_use]
    pub const fn with_limits(mut self, limits: ObjectLimits) -> Self {
        self.limits = limits;
        self
    }

    pub(crate) fn to_json(&self) -> String {
        let hash_json = match &self.hash {
            Some(h) => h.to_json(),
            None => "null".to_string(),
        };
        format!(
            concat!(
                "    {{\n",
                "      \"id\": \"{}\",\n",
                "      \"source\": \"{}\",\n",
                "      \"domain\": \"{}\",\n",
                "      \"object\": \"{}\",\n",
                "      \"kind\": \"{}\",\n",
                "      \"status\": \"{}\",\n",
                "      \"path\": {},\n",
                "      \"bytes\": {},\n",
                "      \"hash\": {},\n",
                "      \"started_at_unix_ns\": {},\n",
                "      \"finished_at_unix_ns\": {},\n",
                "      \"elapsed_us\": {},\n",
                "      \"limits\": {},\n",
                "      \"reason\": {}\n",
                "    }}"
            ),
            json_escape(&self.id),
            json_escape(self.source.as_str()),
            json_escape(&self.domain),
            json_escape(&self.object),
            json_escape(self.kind.as_str()),
            json_escape(self.status.as_str()),
            json_optional_str(&self.path),
            json_optional_u64(self.bytes),
            hash_json,
            json_optional_str(&self.started_at_unix_ns),
            json_optional_str(&self.finished_at_unix_ns),
            json_optional_u64(self.elapsed_us),
            self.limits.to_json(),
            json_optional_str(&self.reason),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_entry_json_contains_required_fields() {
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

        let json = entry.to_json();
        assert!(json.contains("\"id\": \"proc.system.stat\""));
        assert!(json.contains("\"status\": \"captured\""));
        assert!(json.contains("\"path\": \"raw/proc/stat\""));
        assert!(json.contains("\"bytes\": 4096"));
    }

    #[test]
    fn manifest_entry_null_fields_use_json_null() {
        let entry = ManifestEntry::new(
            "proc.not_found",
            SourceSlug::Proc,
            "system",
            "/proc/missing",
            ObjectKind::File,
            ManifestStatus::NotFound,
        )
        .with_reason("file does not exist");

        let json = entry.to_json();
        assert!(json.contains("\"path\": null"));
        assert!(json.contains("\"reason\": \"file does not exist\""));
    }

    #[test]
    fn all_manifest_statuses_serialize_as_expected_strings() {
        let statuses = [
            (ManifestStatus::Captured, "captured"),
            (ManifestStatus::Vanished, "vanished"),
            (ManifestStatus::NotFound, "not_found"),
            (ManifestStatus::PermissionDenied, "permission_denied"),
            (ManifestStatus::Timeout, "timeout"),
            (ManifestStatus::SizeLimited, "size_limited"),
            (ManifestStatus::Truncated, "truncated"),
            (ManifestStatus::SkippedByPolicy, "skipped_by_policy"),
            (ManifestStatus::Unsupported, "unsupported"),
            (ManifestStatus::IoError, "io_error"),
        ];

        for (status, expected_str) in &statuses {
            let entry = ManifestEntry::new(
                format!("test.{expected_str}"),
                SourceSlug::Proc,
                "system",
                "test",
                ObjectKind::File,
                *status,
            );
            let json = entry.to_json();
            let expected = format!("\"status\": \"{expected_str}\"");
            assert!(
                json.contains(&expected),
                "status {expected_str} not found in JSON: {json}"
            );
        }
    }

    #[test]
    fn all_object_kinds_serialize_as_expected_strings() {
        let kinds = [
            (ObjectKind::File, "file"),
            (ObjectKind::FileSet, "file_set"),
            (ObjectKind::DirListing, "dir_listing"),
            (ObjectKind::Symlink, "symlink"),
            (ObjectKind::Metadata, "metadata"),
            (ObjectKind::MetadataOnly, "metadata_only"),
            (ObjectKind::BoundedTree, "bounded_tree"),
            (ObjectKind::EventWindow, "event_window"),
            (ObjectKind::NativeDump, "native_dump"),
        ];

        for (kind, expected_str) in &kinds {
            let entry = ManifestEntry::new(
                format!("test.{expected_str}"),
                SourceSlug::Proc,
                "system",
                "test",
                *kind,
                ManifestStatus::Captured,
            );
            let json = entry.to_json();
            let expected = format!("\"kind\": \"{expected_str}\"");
            assert!(
                json.contains(&expected),
                "kind {expected_str} not found in JSON: {json}"
            );
        }
    }

    #[test]
    fn hash_record_json_format() {
        let hash = HashRecord::new("sha256", "abcdef1234567890");
        let json = hash.to_json();
        assert!(json.contains("\"algorithm\":\"sha256\""));
        assert!(json.contains("\"value\":\"abcdef1234567890\""));
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn manifest_entry_with_all_optional_fields_set() {
        let entry = ManifestEntry::new(
            "full.entry",
            SourceSlug::Proc,
            "domain",
            "object",
            ObjectKind::File,
            ManifestStatus::Captured,
        )
        .with_path("raw/path")
        .with_bytes(1024)
        .with_hash(HashRecord::new("sha256", "deadbeef"))
        .with_timing("1000000000", "2000000000", 1000)
        .with_reason("all fields set")
        .with_limits(ObjectLimits::new(4096, 100, 64, 2));

        let json = entry.to_json();
        assert!(json.contains("\"path\": \"raw/path\""));
        assert!(json.contains("\"bytes\": 1024"));
        assert!(json.contains("\"algorithm\":\"sha256\""));
        assert!(json.contains("\"started_at_unix_ns\": \"1000000000\""));
        assert!(json.contains("\"finished_at_unix_ns\": \"2000000000\""));
        assert!(json.contains("\"elapsed_us\": 1000"));
        assert!(json.contains("\"max_bytes\":4096"));
        assert!(json.contains("\"max_files\":64"));
        assert!(json.contains("\"max_depth\":2"));
        assert!(json.contains("\"reason\": \"all fields set\""));
    }

    #[test]
    fn manifest_entry_with_vanished_status_serializes() {
        let entry = ManifestEntry::new(
            "proc.process.12345.status",
            SourceSlug::Proc,
            "process",
            "12345/status",
            ObjectKind::File,
            ManifestStatus::Vanished,
        )
        .with_reason("process exited during capture");

        let json = entry.to_json();
        assert!(json.contains("\"status\": \"vanished\""));
        assert!(json.contains("\"reason\": \"process exited during capture\""));
    }

    #[test]
    fn object_limits_default_for_small_file() {
        let limits = ObjectLimits::default_for_small_file();
        assert_eq!(limits.max_bytes, 1_048_576);
        assert_eq!(limits.timeout_ms, 50);
        assert_eq!(limits.max_files, 1);
        assert_eq!(limits.max_depth, 0);
        let json = limits.to_json();
        assert!(json.contains("\"max_bytes\":1048576"));
        assert!(json.contains("\"max_files\":1"));
    }
}
