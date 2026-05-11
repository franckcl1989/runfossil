#![forbid(unsafe_code)]
#![doc = "Post-capture packaging skeleton for runfossil."]

/// Default archive extension for future packaged snapshots.
pub const DEFAULT_ARCHIVE_EXTENSION: &str = ".tar.zst";

/// Returns whether packaging should run after capture.
#[must_use]
pub const fn is_post_capture_only() -> bool {
    true
}
