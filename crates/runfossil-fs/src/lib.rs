#![forbid(unsafe_code)]
#![doc = "Bounded filesystem helper skeleton for runfossil collectors."]

use std::path::Path;

use runfossil_core::{ArtifactPathError, validate_relative_artifact_path};

/// Limits for bounded file reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedReadLimits {
    /// Maximum bytes to read.
    pub max_bytes: u64,
    /// Timeout budget in milliseconds.
    pub timeout_ms: u64,
}

impl BoundedReadLimits {
    /// Creates bounded read limits.
    #[must_use]
    pub const fn new(max_bytes: u64, timeout_ms: u64) -> Self {
        Self {
            max_bytes,
            timeout_ms,
        }
    }
}

/// Validates a relative output path before filesystem helpers write artifacts.
pub fn validate_output_path(path: &Path) -> Result<(), ArtifactPathError> {
    validate_relative_artifact_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_paths_cannot_escape_snapshot() {
        assert!(validate_output_path(Path::new("raw/proc/stat")).is_ok());
        assert!(validate_output_path(Path::new("../outside")).is_err());
    }
}
