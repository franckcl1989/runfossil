#![forbid(unsafe_code)]
#![doc = "Snapshot store for runfossil: directory creation, manifest tracking, error logging, and partial snapshot behavior."]

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use runfossil_core::EffectiveUid;

pub mod manifest;
pub mod store;

pub use manifest::{HashRecord, ManifestEntry, ObjectLimits};
pub use store::{ErrorLogEntry, SnapshotStore};

/// Snapshot manifest file name.
pub const MANIFEST_FILE: &str = "manifest.json";

/// Snapshot plan file name.
pub const PLAN_FILE: &str = "plan.json";

/// Snapshot error log file name.
pub const ERRORS_FILE: &str = "errors.jsonl";

/// Snapshot completion marker file name.
pub const COMPLETE_MARKER: &str = "CAPTURE_COMPLETE";

/// Metadata directory name.
pub const META_DIR: &str = "meta";

/// Raw evidence directory name.
pub const RAW_DIR: &str = "raw";

/// Host identity metadata used by the store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostMetadata {
    /// Hostname.
    pub hostname: String,
    /// Kernel boot ID.
    pub boot_id: String,
    /// Kernel release.
    pub kernel_release: String,
    /// Machine architecture.
    pub machine: String,
}

/// Runtime metadata used to create a snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotMetadata {
    /// Producing tool version.
    pub tool_version: String,
    /// Capture start timestamp in Unix nanoseconds as a decimal string.
    pub started_at_unix_ns: String,
    /// Effective UID used by capture.
    pub effective_uid: EffectiveUid,
    /// Host metadata.
    pub host: HostMetadata,
}

/// Store error.
#[derive(Debug)]
pub enum StoreError {
    /// Filesystem I/O failed.
    Io {
        /// Operation context.
        context: &'static str,
        /// Source error.
        source: io::Error,
    },
    /// Output path cannot be represented safely.
    InvalidOutputPath {
        /// Operation context.
        context: &'static str,
    },
    /// The snapshot was already finalized.
    AlreadyFinalized,
}

impl StoreError {
    pub(crate) fn io(context: &'static str, source: io::Error) -> Self {
        Self::Io { context, source }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
            Self::InvalidOutputPath { context } => {
                write!(formatter, "{context}: invalid output path")
            }
            Self::AlreadyFinalized => formatter.write_str("snapshot has already been finalized"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidOutputPath { .. } | Self::AlreadyFinalized => None,
        }
    }
}

pub(crate) fn create_dir(path: &Path) -> Result<(), StoreError> {
    fs::create_dir(path).map_err(|source| StoreError::io("create snapshot directory", source))?;
    set_dir_permissions(path)?;
    Ok(())
}

pub(crate) fn create_dir_all(path: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(path)
        .map_err(|source| StoreError::io("create snapshot directory tree", source))?;
    set_dir_permissions(path)?;
    Ok(())
}

pub(crate) fn write_atomic(path: &Path, contents: &str) -> Result<(), StoreError> {
    let tmp_path = temporary_path(path)?;
    {
        let mut file = File::create(&tmp_path)
            .map_err(|source| StoreError::io("create temporary file", source))?;
        file.write_all(contents.as_bytes())
            .map_err(|source| StoreError::io("write temporary file", source))?;
        file.sync_all()
            .map_err(|source| StoreError::io("sync temporary file", source))?;
    }
    set_file_permissions(&tmp_path)?;
    fs::rename(&tmp_path, path).map_err(|source| StoreError::io("rename snapshot file", source))?;
    Ok(())
}

pub(crate) fn write_atomic_bytes(path: &Path, contents: &[u8]) -> Result<(), StoreError> {
    let tmp_path = temporary_path(path)?;
    {
        let mut file = File::create(&tmp_path)
            .map_err(|source| StoreError::io("create temporary file", source))?;
        file.write_all(contents)
            .map_err(|source| StoreError::io("write temporary file", source))?;
        file.sync_all()
            .map_err(|source| StoreError::io("sync temporary file", source))?;
    }
    set_file_permissions(&tmp_path)?;
    fs::rename(&tmp_path, path).map_err(|source| StoreError::io("rename snapshot file", source))?;
    Ok(())
}

pub(crate) fn append_line(path: &Path, line: &str) -> Result<(), StoreError> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| StoreError::io("open file for append", source))?;
    writeln!(file, "{line}").map_err(|source| StoreError::io("append to file", source))?;
    Ok(())
}

pub(crate) fn temporary_path(path: &Path) -> Result<PathBuf, StoreError> {
    let file_name = path.file_name().and_then(std::ffi::OsStr::to_str).ok_or(
        StoreError::InvalidOutputPath {
            context: "create temporary path",
        },
    )?;
    Ok(path.with_file_name(format!(".{file_name}.tmp")))
}

pub(crate) fn json_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

pub(crate) fn json_optional_str(value: &Option<String>) -> String {
    match value {
        Some(v) => format!("\"{}\"", json_escape(v)),
        None => "null".to_string(),
    }
}

pub(crate) fn json_optional_u64(value: Option<u64>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "null".to_string(),
    }
}

#[cfg(unix)]
fn set_dir_permissions(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|source| StoreError::io("set directory permissions", source))
}

#[cfg(not(unix))]
fn set_dir_permissions(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(unix)]
pub(crate) fn set_file_permissions(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|source| StoreError::io("set file permissions", source))
}

#[cfg(not(unix))]
pub(crate) fn set_file_permissions(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_handles_control_characters() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }

    #[test]
    fn json_optional_str_handles_none() {
        assert_eq!(json_optional_str(&None), "null");
    }
}
