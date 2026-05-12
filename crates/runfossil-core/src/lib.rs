#![forbid(unsafe_code)]
#![doc = "Shared domain model and invariants for runfossil."]

use std::fmt;
use std::path::{Component, Path};

/// Effective user ID observed by the capture entry point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveUid(u32);

impl EffectiveUid {
    /// Effective UID for root.
    pub const ROOT: Self = Self(0);

    /// Creates an effective UID value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the numeric effective UID.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Returns whether this UID is root.
    #[must_use]
    pub const fn is_root(self) -> bool {
        self.0 == Self::ROOT.0
    }
}

impl fmt::Display for EffectiveUid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Stable source family slug used in snapshot control files.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceSlug {
    /// `/proc`.
    Proc,
    /// `/sys`.
    Sys,
    /// `/run`.
    Run,
    /// `/dev`.
    Dev,
    /// Kernel ring buffer.
    Kernel,
    /// System event log store.
    Logs,
    /// Service manager.
    Service,
    /// Netlink.
    Netlink,
    /// Security and audit subsystem.
    Security,
    /// User and session database.
    Sessions,
    /// Scheduler and job runtime.
    Scheduler,
    /// Time synchronization subsystem.
    Time,
    /// Crash dump store.
    Crash,
    /// Hardware management interface.
    Hardware,
}

impl SourceSlug {
    /// Returns the canonical source slug.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proc => "proc",
            Self::Sys => "sys",
            Self::Run => "run",
            Self::Dev => "dev",
            Self::Kernel => "kernel",
            Self::Logs => "logs",
            Self::Service => "service",
            Self::Netlink => "netlink",
            Self::Security => "security",
            Self::Sessions => "sessions",
            Self::Scheduler => "scheduler",
            Self::Time => "time",
            Self::Crash => "crash",
            Self::Hardware => "hardware",
        }
    }

    /// Returns all baseline source slugs.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Proc,
            Self::Sys,
            Self::Run,
            Self::Dev,
            Self::Netlink,
            Self::Kernel,
            Self::Logs,
            Self::Service,
            Self::Security,
            Self::Sessions,
            Self::Scheduler,
            Self::Time,
            Self::Crash,
            Self::Hardware,
        ]
    }
}

impl fmt::Display for SourceSlug {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Static coverage decision from the Coverage Decision Matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageDecision {
    /// Required target.
    Collect,
    /// Valid target selected only when source, support, and budget allow.
    Conditional,
    /// Valid target with deliberate bounds.
    Limited,
    /// Valid target waiting for native implementation.
    DeferredNative,
    /// Intentionally outside collection.
    Exclude,
}

impl CoverageDecision {
    /// Returns the manifest/control-file spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Collect => "collect",
            Self::Conditional => "conditional",
            Self::Limited => "limited",
            Self::DeferredNative => "deferred-native",
            Self::Exclude => "exclude",
        }
    }
}

impl fmt::Display for CoverageDecision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Runtime planner decision recorded in `plan.json`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanDecision {
    /// Run with normal limits for the coverage unit.
    Scheduled,
    /// Run with stricter limits or selected-object policy.
    Limited,
    /// Deliberately skipped for policy, cost, pressure, or dependency reasons.
    SkippedByPolicy,
    /// Valid source, but this implementation has no native collector.
    Unsupported,
    /// Probe showed that the source or object is absent on this host.
    NotPresent,
}

impl PlanDecision {
    /// Returns the manifest/control-file spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Limited => "limited",
            Self::SkippedByPolicy => "skipped_by_policy",
            Self::Unsupported => "unsupported",
            Self::NotPresent => "not_present",
        }
    }
}

impl fmt::Display for PlanDecision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Final manifest status recorded for a raw snapshot object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestStatus {
    /// The object was captured within assigned limits.
    Captured,
    /// The object disappeared before or during capture.
    Vanished,
    /// The object was not present on this host.
    NotFound,
    /// Root was denied by kernel, LSM, or mount policy.
    PermissionDenied,
    /// The task exceeded its time budget.
    Timeout,
    /// Content was not captured because it exceeded limits.
    SizeLimited,
    /// Partial content was intentionally written within limits.
    Truncated,
    /// The planner intentionally skipped this object.
    SkippedByPolicy,
    /// The source exists, but no native collector is implemented.
    Unsupported,
    /// An unexpected I/O error occurred.
    IoError,
}

impl ManifestStatus {
    /// Returns the manifest/control-file spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Captured => "captured",
            Self::Vanished => "vanished",
            Self::NotFound => "not_found",
            Self::PermissionDenied => "permission_denied",
            Self::Timeout => "timeout",
            Self::SizeLimited => "size_limited",
            Self::Truncated => "truncated",
            Self::SkippedByPolicy => "skipped_by_policy",
            Self::Unsupported => "unsupported",
            Self::IoError => "io_error",
        }
    }
}

impl fmt::Display for ManifestStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Capture task priority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Priority {
    /// Core global runtime evidence.
    P0,
    /// Process and immediate runtime summary evidence.
    P1,
    /// Subsystem runtime evidence.
    P2,
    /// Conditional deep evidence.
    P3,
    /// High-cost or high-risk evidence.
    P4,
    /// Not scheduled for collection.
    NotApplicable,
}

impl Priority {
    /// Returns the plan/control-file spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
            Self::P4 => "P4",
            Self::NotApplicable => "NA",
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Raw object kind recorded in `manifest.json`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectKind {
    /// Single file-like payload.
    File,
    /// A set of related file-like payloads.
    FileSet,
    /// Directory listing.
    DirListing,
    /// Symlink target.
    Symlink,
    /// Metadata record.
    Metadata,
    /// Metadata without payload content.
    MetadataOnly,
    /// Bounded directory tree.
    BoundedTree,
    /// Bounded event window.
    EventWindow,
    /// Native protocol dump.
    NativeDump,
}

impl ObjectKind {
    /// Returns the manifest spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::FileSet => "file_set",
            Self::DirListing => "dir_listing",
            Self::Symlink => "symlink",
            Self::Metadata => "metadata",
            Self::MetadataOnly => "metadata_only",
            Self::BoundedTree => "bounded_tree",
            Self::EventWindow => "event_window",
            Self::NativeDump => "native_dump",
        }
    }
}

impl fmt::Display for ObjectKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Relative artifact path validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactPathError {
    /// The path is absolute.
    Absolute,
    /// The path contains `..`.
    ParentComponent,
    /// The path contains a platform prefix.
    Prefix,
}

impl fmt::Display for ArtifactPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute => formatter.write_str("artifact path must be relative"),
            Self::ParentComponent => formatter.write_str("artifact path must not contain '..'"),
            Self::Prefix => formatter.write_str("artifact path must not contain a path prefix"),
        }
    }
}

impl std::error::Error for ArtifactPathError {}

/// Validates that a snapshot artifact path is relative and cannot escape the snapshot.
///
/// Empty paths are allowed here because some callers validate optional paths before
/// deciding whether to write an artifact. Callers that require a non-empty path should
/// add that rule at their boundary.
pub fn validate_relative_artifact_path(path: &Path) -> Result<(), ArtifactPathError> {
    if path.is_absolute() {
        return Err(ArtifactPathError::Absolute);
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(ArtifactPathError::ParentComponent),
            Component::RootDir => return Err(ArtifactPathError::Absolute),
            Component::Prefix(_) => return Err(ArtifactPathError::Prefix),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_paths_reject_escape_attempts() {
        assert!(validate_relative_artifact_path(Path::new("raw/proc/stat")).is_ok());
        assert_eq!(
            validate_relative_artifact_path(Path::new("../raw/proc/stat")),
            Err(ArtifactPathError::ParentComponent)
        );
        assert_eq!(
            validate_relative_artifact_path(Path::new("/raw/proc/stat")),
            Err(ArtifactPathError::Absolute)
        );
    }

    #[test]
    fn vocabulary_spellings_match_snapshot_contract() {
        assert_eq!(CoverageDecision::DeferredNative.as_str(), "deferred-native");
        assert_eq!(PlanDecision::SkippedByPolicy.as_str(), "skipped_by_policy");
        assert_eq!(
            ManifestStatus::PermissionDenied.as_str(),
            "permission_denied"
        );
        assert_eq!(Priority::NotApplicable.as_str(), "NA");
        assert_eq!(ObjectKind::NativeDump.as_str(), "native_dump");
        assert_eq!(SourceSlug::Hardware.as_str(), "hardware");
    }

    #[test]
    fn coverage_decision_all_variants_have_nonempty_str() {
        let variants = [
            CoverageDecision::Collect,
            CoverageDecision::Limited,
            CoverageDecision::Exclude,
            CoverageDecision::DeferredNative,
            CoverageDecision::Conditional,
        ];
        for v in &variants {
            assert!(!v.as_str().is_empty());
        }
    }

    #[test]
    fn plan_decision_all_variants_have_nonempty_str() {
        let variants = [
            PlanDecision::Scheduled,
            PlanDecision::Limited,
            PlanDecision::SkippedByPolicy,
            PlanDecision::NotPresent,
            PlanDecision::Unsupported,
        ];
        for v in &variants {
            assert!(!v.as_str().is_empty());
        }
    }

    #[test]
    fn manifest_status_all_variants_have_nonempty_str() {
        let variants = [
            ManifestStatus::Captured,
            ManifestStatus::Vanished,
            ManifestStatus::NotFound,
            ManifestStatus::PermissionDenied,
            ManifestStatus::Timeout,
            ManifestStatus::SizeLimited,
            ManifestStatus::Truncated,
            ManifestStatus::SkippedByPolicy,
            ManifestStatus::Unsupported,
            ManifestStatus::IoError,
        ];
        for v in &variants {
            assert!(!v.as_str().is_empty());
        }
    }

    #[test]
    fn priority_all_variants_have_nonempty_str() {
        let variants = [
            Priority::P0,
            Priority::P1,
            Priority::P2,
            Priority::P3,
            Priority::P4,
            Priority::NotApplicable,
        ];
        for v in &variants {
            assert!(!v.as_str().is_empty());
        }
    }

    #[test]
    fn object_kind_all_variants_have_nonempty_str() {
        let variants = [
            ObjectKind::File,
            ObjectKind::FileSet,
            ObjectKind::DirListing,
            ObjectKind::Symlink,
            ObjectKind::Metadata,
            ObjectKind::MetadataOnly,
            ObjectKind::BoundedTree,
            ObjectKind::EventWindow,
            ObjectKind::NativeDump,
        ];
        for v in &variants {
            assert!(!v.as_str().is_empty());
        }
    }

    #[test]
    fn source_slug_all_variants_have_nonempty_str() {
        for variant in SourceSlug::all() {
            assert!(!variant.as_str().is_empty());
        }
    }

    #[test]
    fn source_slug_all_returns_fourteen_variants() {
        assert_eq!(SourceSlug::all().len(), 14);
    }

    #[test]
    fn effective_uid_root_is_zero() {
        assert_eq!(EffectiveUid::ROOT.get(), 0);
        assert!(EffectiveUid::ROOT.is_root());
    }

    #[test]
    fn effective_uid_non_root_is_not_root() {
        let uid = EffectiveUid::new(1000);
        assert_eq!(uid.get(), 1000);
        assert!(!uid.is_root());
    }

    #[test]
    fn effective_uid_display_formats_as_number() {
        assert_eq!(format!("{}", EffectiveUid::new(1000)), "1000");
    }

    #[test]
    fn artifact_path_rejects_dot_dot_only() {
        assert_eq!(
            validate_relative_artifact_path(Path::new("..")),
            Err(ArtifactPathError::ParentComponent)
        );
    }

    #[test]
    fn artifact_path_accepts_current_dir() {
        assert!(validate_relative_artifact_path(Path::new(".")).is_ok());
    }

    #[test]
    fn artifact_path_accepts_empty() {
        assert!(validate_relative_artifact_path(Path::new("")).is_ok());
    }

    #[test]
    fn artifact_path_rejects_prefix_on_windows_style_paths() {
        #[cfg(windows)]
        {
            assert_eq!(
                validate_relative_artifact_path(Path::new("C:\\windows\\path")),
                Err(ArtifactPathError::Prefix)
            );
        }
        #[cfg(not(windows))]
        {
            assert!(validate_relative_artifact_path(Path::new("any/relative/path")).is_ok());
        }
    }

    #[test]
    fn artifact_path_error_display() {
        assert_eq!(
            format!("{}", ArtifactPathError::Absolute),
            "artifact path must be relative"
        );
        assert_eq!(
            format!("{}", ArtifactPathError::ParentComponent),
            "artifact path must not contain '..'"
        );
        assert_eq!(
            format!("{}", ArtifactPathError::Prefix),
            "artifact path must not contain a path prefix"
        );
    }
}
