#![forbid(unsafe_code)]
#![doc = "Bounded filesystem helpers for runfossil collectors."]

use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

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

    /// Default limits for small proc/sys files.
    #[must_use]
    pub const fn small() -> Self {
        Self {
            max_bytes: 1_048_576,
            timeout_ms: 100,
        }
    }

    /// Default limits for a single process summary file.
    #[must_use]
    pub const fn per_process() -> Self {
        Self {
            max_bytes: 65_536,
            timeout_ms: 100,
        }
    }
}

/// Limits for bounded directory traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedTraversalLimits {
    /// Maximum entries to return.
    pub max_files: u64,
    /// Maximum depth when visiting subtrees.
    pub max_depth: u32,
    /// Timeout budget in milliseconds.
    pub timeout_ms: u64,
}

impl BoundedTraversalLimits {
    /// Creates bounded traversal limits.
    #[must_use]
    pub const fn new(max_files: u64, max_depth: u32, timeout_ms: u64) -> Self {
        Self {
            max_files,
            max_depth,
            timeout_ms,
        }
    }

    /// Default limits for process listing.
    #[must_use]
    pub const fn process_listing() -> Self {
        Self {
            max_files: 32_768,
            max_depth: 1,
            timeout_ms: 1_000,
        }
    }

    /// Default limits for bounded tree capture.
    #[must_use]
    pub const fn bounded_tree() -> Self {
        Self {
            max_files: 4_096,
            max_depth: 4,
            timeout_ms: 2_000,
        }
    }
}

/// Result of a bounded file read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadResult {
    /// File content bytes.
    pub content: Vec<u8>,
    /// Whether the file was larger than `max_bytes`.
    pub was_truncated: bool,
}

impl ReadResult {
    /// Returns the content as a UTF-8 string lossily.
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.content).into_owned()
    }
}

/// Result of listing a directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListResult {
    /// Directory entries, sorted by name.
    pub entries: Vec<DirEntry>,
    /// Whether the listing was truncated at `max_files`.
    pub was_truncated: bool,
}

/// A directory entry discovered during traversal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirEntry {
    /// Entry name (not a full path).
    pub name: String,
    /// Whether the entry is a directory.
    pub is_dir: bool,
}

/// Filesystem error returned by bounded helpers.
#[derive(Debug)]
pub enum FsError {
    /// Path does not exist.
    NotFound(PathBuf),
    /// Permission denied.
    PermissionDenied(PathBuf),
    /// Read timed out.
    Timeout(PathBuf),
    /// Content exceeded the byte limit.
    SizeLimited {
        /// The file path.
        path: PathBuf,
        /// The byte limit that was exceeded.
        max_bytes: u64,
    },
    /// Unexpected I/O error.
    Io {
        /// The file path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
}

impl FsError {
    #[must_use]
    fn not_found(path: impl AsRef<Path>) -> Self {
        Self::NotFound(path.as_ref().to_path_buf())
    }

    #[must_use]
    fn permission_denied(path: impl AsRef<Path>) -> Self {
        Self::PermissionDenied(path.as_ref().to_path_buf())
    }

    #[must_use]
    #[allow(dead_code)]
    fn timeout(path: impl AsRef<Path>) -> Self {
        Self::Timeout(path.as_ref().to_path_buf())
    }

    #[must_use]
    fn io(path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Io {
            path: path.as_ref().to_path_buf(),
            source,
        }
    }

    #[must_use]
    fn map_io_error(path: impl AsRef<Path>, error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::not_found(path),
            io::ErrorKind::PermissionDenied => Self::permission_denied(path),
            io::ErrorKind::TimedOut => Self::timeout(path),
            _ => Self::io(path, error),
        }
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(path) => write!(formatter, "not found: {}", path.display()),
            Self::PermissionDenied(path) => {
                write!(formatter, "permission denied: {}", path.display())
            }
            Self::Timeout(path) => write!(formatter, "timeout: {}", path.display()),
            Self::SizeLimited { path, max_bytes } => {
                write!(
                    formatter,
                    "file exceeded {} bytes: {}",
                    max_bytes,
                    path.display()
                )
            }
            Self::Io { path, source } => write!(formatter, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for FsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Validates a relative output path before filesystem helpers write artifacts.
pub fn validate_output_path(path: &Path) -> Result<(), ArtifactPathError> {
    validate_relative_artifact_path(path)
}

/// Reads a file with a byte limit.
///
/// Returns `ReadResult` with `was_truncated` set to `true` if the file content
/// exceeded `max_bytes`. The returned content contains at most `max_bytes` + 1
/// bytes so callers can detect truncation without reading the whole file.
pub fn read_file_bounded(path: &Path, limits: BoundedReadLimits) -> Result<ReadResult, FsError> {
    let mut file = fs::File::open(path).map_err(|error| FsError::map_io_error(path, error))?;

    let mut content = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut total_read: u64 = 0;

    loop {
        let max_read = (limits.max_bytes + 1).saturating_sub(total_read);
        if max_read == 0 {
            return Ok(ReadResult {
                content,
                was_truncated: true,
            });
        }

        let chunk_size = (max_read as usize).min(buffer.len());
        match file.read(&mut buffer[..chunk_size]) {
            Ok(0) => break,
            Ok(bytes_read) => {
                total_read += bytes_read as u64;
                content.extend_from_slice(&buffer[..bytes_read]);
            }
            Err(error) => return Err(FsError::map_io_error(path, error)),
        }
    }

    Ok(ReadResult {
        was_truncated: false,
        content,
    })
}

/// Reads a symlink target path.
///
/// Falls back to returning a zero-length path when the link cannot be read.
/// Callers should check the returned path length or use the error variant.
pub fn read_link_bounded(path: &Path, limits: BoundedReadLimits) -> Result<PathBuf, FsError> {
    let _ = limits;
    fs::read_link(path).map_err(|error| FsError::map_io_error(path, error))
}

/// Lists directory entries with a count limit.
///
/// Entries are sorted by name. `was_truncated` is set when the number of entries
/// exceeds `max_files`.
pub fn list_dir_entries(
    path: &Path,
    limits: BoundedTraversalLimits,
) -> Result<ListResult, FsError> {
    let dir = match fs::read_dir(path) {
        Ok(dir) => dir,
        Err(error) => return Err(FsError::map_io_error(path, error)),
    };

    let mut entries = Vec::new();
    let mut count: u64 = 0;

    for entry_result in dir {
        if count >= limits.max_files {
            return Ok(ListResult {
                entries,
                was_truncated: true,
            });
        }

        let entry = match entry_result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

        entries.push(DirEntry { name, is_dir });
        count += 1;
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ListResult {
        entries,
        was_truncated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_paths_cannot_escape_snapshot() {
        assert!(validate_output_path(Path::new("raw/proc/stat")).is_ok());
        assert!(validate_output_path(Path::new("../outside")).is_err());
    }

    #[test]
    fn read_file_bounded_returns_content() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("test.txt");
        fs::write(&path, b"hello").expect("write");

        let result =
            read_file_bounded(&path, BoundedReadLimits::small()).expect("read_file_bounded");
        assert_eq!(result.content, b"hello");
        assert!(!result.was_truncated);
    }

    #[test]
    fn read_file_bounded_detects_truncation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("test.txt");
        fs::write(&path, b"hello world").expect("write");

        let limit = BoundedReadLimits::new(5, 100);
        let result = read_file_bounded(&path, limit).expect("read_file_bounded");
        assert_eq!(result.content, b"hello ");
        assert!(result.was_truncated);
    }

    #[test]
    fn read_file_bounded_returns_not_found_for_missing_file() {
        let missing = Path::new("/nonexistent/file");
        let err = read_file_bounded(missing, BoundedReadLimits::small()).unwrap_err();
        assert!(matches!(
            err,
            FsError::NotFound(_) | FsError::PermissionDenied(_) | FsError::Io { .. }
        ));
    }

    #[test]
    fn list_dir_entries_returns_sorted_entries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::write(tmp.path().join("b.txt"), b"b").expect("write");
        fs::write(tmp.path().join("a.txt"), b"a").expect("write");
        fs::create_dir(tmp.path().join("sub")).expect("mkdir");

        let result = list_dir_entries(tmp.path(), BoundedTraversalLimits::bounded_tree())
            .expect("list_dir_entries");
        let names: Vec<_> = result.entries.iter().map(|e| &*e.name).collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "sub"]);
        assert!(!result.was_truncated);

        let sub_entry = result
            .entries
            .iter()
            .find(|e| e.name == "sub")
            .expect("sub dir");
        assert!(sub_entry.is_dir);
    }

    #[test]
    fn list_dir_entries_truncates_at_max_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::write(tmp.path().join("a"), b"").expect("write");
        fs::write(tmp.path().join("b"), b"").expect("write");
        fs::write(tmp.path().join("c"), b"").expect("write");

        let limits = BoundedTraversalLimits::new(2, 1, 100);
        let result = list_dir_entries(tmp.path(), limits).expect("list_dir_entries");
        assert_eq!(result.entries.len(), 2);
        assert!(result.was_truncated);
    }
}
