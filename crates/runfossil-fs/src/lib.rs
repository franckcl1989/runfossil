#![forbid(unsafe_code)]
#![doc = "Bounded filesystem helpers for runfossil collectors."]

use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
    let deadline = Instant::now() + Duration::from_millis(limits.timeout_ms);

    loop {
        if Instant::now() >= deadline {
            return Err(FsError::timeout(path));
        }
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

/// Reads a symlink target path with limits recorded in the manifest.
pub fn read_link_bounded(path: &Path, _limits: BoundedReadLimits) -> Result<PathBuf, FsError> {
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
    let deadline = Instant::now() + Duration::from_millis(limits.timeout_ms);

    for entry_result in dir {
        if Instant::now() >= deadline {
            return Err(FsError::timeout(path));
        }
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

/// Configuration for auto-discovery collection of a directory.
#[derive(Clone, Debug)]
pub struct AutoDiscoverConfig {
    /// Entry names (not full paths) to exclude from collection.
    pub blacklist: &'static [&'static str],
    /// Max bytes per individual file.
    pub max_bytes_per_file: u64,
    /// Max files to collect per directory level.
    pub max_files_per_level: u64,
    /// Max traversal depth (0 = single level only).
    pub max_depth: u32,
    /// Timeout per directory in milliseconds.
    pub timeout_ms: u64,
    /// If true, recurse into subdirectories.
    pub recurse: bool,
    /// If true, skip subdirectory entries (only collect files).
    pub files_only: bool,
}

impl AutoDiscoverConfig {
    /// Default config for /proc global files: single level, no recursion,
    /// excludes known dangerous entries.
    #[must_use]
    pub const fn proc_global() -> Self {
        Self {
            blacklist: &[
                "kcore",
                "kallsyms",
                "kpagecount",
                "kpageflags",
                "kpagecgroup",
                "sysrq-trigger",
            ],
            max_bytes_per_file: 1_048_576,
            max_files_per_level: 512,
            max_depth: 0,
            timeout_ms: 200,
            recurse: false,
            files_only: true,
        }
    }

    /// Config for /proc/<pid> per-process: single level, all files except
    /// dangerous/unbounded entries. Memory-mapping files (smaps, maps,
    /// numa_maps, smaps_rollup) are excluded from auto-discovery because
    /// their cumulative size on hosts with many processes can exceed the
    /// capture byte budget; they remain tracked as coverage units for
    /// planned explicit collection.
    #[must_use]
    pub const fn proc_per_process() -> Self {
        Self {
            blacklist: &[
                "mem",
                "pagemap",
                "clear_refs",
                "oom_adj",
                "attr",
                "coredump_filter",
                "uid_map",
                "gid_map",
                "projid_map",
                "setgroups",
                "reclaim",
                "smaps",
                "smaps_rollup",
                "maps",
                "numa_maps",
            ],
            max_bytes_per_file: 131_072,
            max_files_per_level: 128,
            max_depth: 0,
            timeout_ms: 500,
            recurse: false,
            files_only: true,
        }
    }

    /// Config for /sys device class directories: bounded recursion, exclude
    /// write interfaces.
    #[must_use]
    pub const fn sys_device_class() -> Self {
        Self {
            blacklist: &[
                "uevent", "bind", "unbind", "probe", "reset", "trigger", "store", "config",
            ],
            max_bytes_per_file: 1_048_576,
            max_files_per_level: 256,
            max_depth: 4,
            timeout_ms: 500,
            recurse: true,
            files_only: false,
        }
    }

    /// Config for /run directories: bounded recursion, cautious depth.
    /// Excludes application-layer container engine sockets per coverage matrix.
    #[must_use]
    pub const fn run_dir() -> Self {
        Self {
            blacklist: &[
                "docker.sock",
                "containerd",
                "crio",
                "runc",
                "podman",
                "kata-containers",
                "gvisor",
            ],
            max_bytes_per_file: 262_144,
            max_files_per_level: 64,
            max_depth: 3,
            timeout_ms: 3000,
            recurse: true,
            files_only: false,
        }
    }

    /// Config for /proc/net: single level, all files.
    #[must_use]
    pub const fn proc_net() -> Self {
        Self {
            blacklist: &[],
            max_bytes_per_file: 262_144,
            max_files_per_level: 256,
            max_depth: 0,
            timeout_ms: 500,
            recurse: false,
            files_only: true,
        }
    }

    /// Returns whether an entry name is blacklisted.
    #[must_use]
    pub fn is_blacklisted(&self, name: &str) -> bool {
        self.blacklist
            .iter()
            .any(|b| name == *b || name.starts_with(&format!("{b}/")))
    }
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
    fn read_file_bounded_returns_content() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("test.txt");
        fs::write(&path, b"hello")?;

        let result = read_file_bounded(&path, BoundedReadLimits::small())?;
        assert_eq!(result.content, b"hello");
        assert!(!result.was_truncated);
        Ok(())
    }

    #[test]
    fn read_file_bounded_detects_truncation() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("test.txt");
        fs::write(&path, b"hello world")?;

        let limit = BoundedReadLimits::new(5, 100);
        let result = read_file_bounded(&path, limit)?;
        assert_eq!(result.content, b"hello ");
        assert!(result.was_truncated);
        Ok(())
    }

    #[test]
    fn read_file_bounded_returns_not_found_for_missing_file() {
        let missing = Path::new("/nonexistent/file");
        match read_file_bounded(missing, BoundedReadLimits::small()) {
            Ok(_) => panic!("missing file should not be read successfully"),
            Err(error) => assert!(matches!(
                error,
                FsError::NotFound(_) | FsError::PermissionDenied(_) | FsError::Io { .. }
            )),
        }
    }

    #[test]
    fn list_dir_entries_returns_sorted_entries() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        fs::write(tmp.path().join("b.txt"), b"b")?;
        fs::write(tmp.path().join("a.txt"), b"a")?;
        fs::create_dir(tmp.path().join("sub"))?;

        let result = list_dir_entries(tmp.path(), BoundedTraversalLimits::bounded_tree())?;
        let names: Vec<_> = result.entries.iter().map(|e| &*e.name).collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "sub"]);
        assert!(!result.was_truncated);

        assert!(result.entries.iter().any(|e| e.name == "sub" && e.is_dir));
        Ok(())
    }

    #[test]
    fn list_dir_entries_truncates_at_max_files() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        fs::write(tmp.path().join("a"), b"")?;
        fs::write(tmp.path().join("b"), b"")?;
        fs::write(tmp.path().join("c"), b"")?;

        let limits = BoundedTraversalLimits::new(2, 1, 100);
        let result = list_dir_entries(tmp.path(), limits)?;
        assert_eq!(result.entries.len(), 2);
        assert!(result.was_truncated);
        Ok(())
    }

    #[test]
    fn list_dir_entries_with_large_count_does_not_truncate_prematurely()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let count = 500;
        for i in 0..count {
            fs::write(tmp.path().join(format!("file_{i:04}")), b"")?;
        }

        let limits = BoundedTraversalLimits::new(count, 1, 5_000);
        let result = list_dir_entries(tmp.path(), limits)?;
        assert_eq!(result.entries.len() as u64, count);
        assert!(!result.was_truncated);
        Ok(())
    }

    #[test]
    fn bounded_traversal_process_listing_is_configured_for_large_hosts() {
        let limits = BoundedTraversalLimits::process_listing();
        assert!(limits.max_files >= 32_767);
        assert_eq!(limits.max_depth, 1);
    }

    #[test]
    fn read_file_bounded_works_with_max_bytes_larger_than_content()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let content = &[0u8; 500];
        fs::write(tmp.path().join("data.bin"), content)?;

        let limits = BoundedReadLimits::new(600, 100);
        let result = read_file_bounded(&tmp.path().join("data.bin"), limits)?;
        assert_eq!(result.content.len(), 500);
        assert!(!result.was_truncated);
        Ok(())
    }
}
