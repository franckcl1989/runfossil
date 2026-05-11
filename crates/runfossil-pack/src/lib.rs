#![forbid(unsafe_code)]
#![doc = "Post-capture packaging and inspection for runfossil."]

pub mod inspect;
pub mod pack;

pub use inspect::{InspectionError, InspectionReport, inspect_snapshot};
pub use pack::{ArchiveMetadata, pack_snapshot, write_archive_metadata};

/// Default archive extension for packaged snapshots.
pub const DEFAULT_ARCHIVE_EXTENSION: &str = ".tar.zst";

/// Returns whether packaging should run after capture.
#[must_use]
pub const fn is_post_capture_only() -> bool {
    true
}
