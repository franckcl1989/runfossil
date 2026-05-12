#![forbid(unsafe_code)]
#![doc = "Post-capture snapshot packaging: builds a .tar.zst archive from a snapshot directory."]

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use tar::Builder;

use runfossil_store::{COMPLETE_MARKER, MANIFEST_FILE};

/// Metadata describing a packaged snapshot archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveMetadata {
    /// Absolute or relative path to the source snapshot directory.
    pub source_dir: PathBuf,
    /// Absolute or relative path to the created archive file.
    pub archive_path: PathBuf,
    /// Packaging time in Unix nanoseconds.
    pub packaging_time_unix_ns: u128,
    /// Compression format identifier.
    pub compression_format: String,
    /// SHA-256 hex digest of the compressed archive.
    pub archive_sha256: String,
    /// Whether the snapshot had a CAPTURE_COMPLETE marker at packaging time.
    pub snapshot_was_complete: bool,
    /// Total compressed archive size in bytes.
    pub archive_size_bytes: u64,
}

/// Pack a snapshot directory into a `.tar.zst` archive.
///
/// The archive is created alongside the snapshot directory (in the parent
/// directory) using the snapshot directory name as the archive stem. Raw
/// evidence content is preserved exactly.
///
/// # Errors
///
/// Returns `io::Error` if the directory does not exist, is not a valid
/// snapshot, or if archive creation fails for any reason.
pub fn pack_snapshot(snapshot_dir: &Path) -> io::Result<ArchiveMetadata> {
    if !snapshot_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("snapshot directory not found: {}", snapshot_dir.display()),
        ));
    }

    if !snapshot_dir.join(MANIFEST_FILE).is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "not a snapshot directory (missing {}): {}",
                MANIFEST_FILE,
                snapshot_dir.display()
            ),
        ));
    }

    let snapshot_was_complete = snapshot_dir.join(COMPLETE_MARKER).is_file();

    let archive_stem = snapshot_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "snapshot directory name is not valid UTF-8",
            )
        })?;
    let archive_name = format!("{archive_stem}.tar.zst");
    let archive_path = snapshot_dir
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&archive_name);

    let packaging_time_unix_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();

    let uncompressed = build_tar(snapshot_dir)?;
    let compressed = zstd::encode_all(&uncompressed[..], 3).map_err(io::Error::other)?;

    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let archive_sha256 = format!("{:x}", hasher.finalize());

    fs::write(&archive_path, &compressed)?;

    Ok(ArchiveMetadata {
        source_dir: snapshot_dir.to_path_buf(),
        archive_path,
        packaging_time_unix_ns,
        compression_format: "zstd".to_string(),
        archive_sha256,
        snapshot_was_complete,
        archive_size_bytes: compressed.len() as u64,
    })
}

/// Builds an uncompressed tar byte buffer from a snapshot directory.
///
/// All regular files and directories under the snapshot root are included
/// with deterministic headers. Files that cannot be read are skipped and
/// reported as warnings. Symbolic links, device nodes, and other special
/// files are skipped.
fn build_tar(snapshot_dir: &Path) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    {
        let mut builder = Builder::new(&mut buffer);
        append_dir_entries(&mut builder, snapshot_dir, snapshot_dir)?;
        builder.finish().map_err(io::Error::other)?;
    }
    Ok(buffer)
}

/// Recursively append directory entries to a tar builder.
fn append_dir_entries<W: Write>(
    builder: &mut Builder<W>,
    dir: &Path,
    base: &Path,
) -> io::Result<()> {
    let read_dir = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            return Err(io::Error::other(format!(
                "cannot read directory {}: {err}",
                dir.display()
            )));
        }
    };

    let mut entries = BTreeMap::new();
    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!(
                    "runfossil-pack: warning: cannot read directory entry in {}: {err}",
                    dir.display()
                );
                continue;
            }
        };
        let name = entry.file_name();
        entries.insert(name, entry);
    }

    for (_name, entry) in entries {
        let path = entry.path();
        let relative = match path.strip_prefix(base) {
            Ok(rel) => rel,
            Err(err) => {
                eprintln!(
                    "runfossil-pack: warning: cannot compute relative path for {}: {err}",
                    path.display()
                );
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(err) => {
                eprintln!(
                    "runfossil-pack: warning: cannot read metadata for {}: {err}",
                    path.display()
                );
                continue;
            }
        };

        let file_type = metadata.file_type();

        if file_type.is_dir() {
            if let Err(err) = builder.append_dir(relative, &path) {
                eprintln!(
                    "runfossil-pack: warning: cannot append directory {}: {err}",
                    relative.display()
                );
                continue;
            }
            append_dir_entries(builder, &path, base)?;
        } else if file_type.is_file()
            && let Err(err) = builder.append_path_with_name(&path, relative)
        {
            eprintln!(
                "runfossil-pack: warning: cannot append file {}: {err}",
                relative.display()
            );
        }
    }
    Ok(())
}

/// Writes archive metadata as JSON alongside the archive.
///
/// Creates a `<archive-stem>.meta.json` file next to the archive.
pub fn write_archive_metadata(metadata: &ArchiveMetadata) -> io::Result<()> {
    let meta_path = metadata.archive_path.with_extension("meta.json");
    let json = archive_metadata_json(metadata);
    fs::write(&meta_path, json)
}

fn archive_metadata_json(metadata: &ArchiveMetadata) -> String {
    let source = metadata.source_dir.display();
    let archive = metadata.archive_path.display();
    format!(
        "{{\n  \"source_dir\": \"{source}\",\n  \"archive_path\": \"{archive}\",\n  \
         \"packaging_time_unix_ns\": \"{}\",\n  \"compression_format\": \"{}\",\n  \
         \"archive_sha256\": \"{}\",\n  \"snapshot_was_complete\": {},\n  \
         \"archive_size_bytes\": {}\n}}\n",
        metadata.packaging_time_unix_ns,
        metadata.compression_format,
        metadata.archive_sha256,
        metadata.snapshot_was_complete,
        metadata.archive_size_bytes,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use runfossil_store::{META_DIR, RAW_DIR};
    use std::io::Read;

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_nanos(),
            Err(_) => 0,
        };
        std::env::temp_dir().join(format!(
            "runfossil-pack-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn create_test_snapshot(dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir.join(META_DIR))?;
        fs::create_dir_all(dir.join(RAW_DIR).join("proc"))?;
        fs::create_dir_all(dir.join(RAW_DIR).join("sys"))?;

        let manifest = concat!(
            "{\n  \"schema_version\": 1,\n  \"tool\": \"runfossil\",\n",
            "  \"objects\": [\n",
            "    {\n      \"id\": \"proc.system.stat\",\n",
            "      \"source\": \"proc\",\n      \"domain\": \"system\",\n",
            "      \"object\": \"/proc/stat\",\n      \"kind\": \"file\",\n",
            "      \"status\": \"captured\"\n    }\n  ]\n}\n"
        );
        fs::write(dir.join(MANIFEST_FILE), manifest)?;
        fs::write(dir.join(COMPLETE_MARKER), "")?;
        fs::write(dir.join(RAW_DIR).join("proc/stat"), b"cpu  0 0 0\n")?;
        fs::write(dir.join(RAW_DIR).join("sys/devices"), b"device data\n")?;
        fs::write(dir.join("errors.jsonl"), "{\"error\": \"test\"}\n")?;
        Ok(())
    }

    #[test]
    fn pack_produces_archive_for_valid_snapshot() {
        let dir = unique_test_dir("pack-valid");
        create_test_snapshot(&dir).expect("create test snapshot");

        let metadata = pack_snapshot(&dir).expect("pack snapshot");
        assert!(metadata.archive_path.is_file());
        assert!(!metadata.archive_sha256.is_empty());
        assert!(metadata.snapshot_was_complete);
        assert!(metadata.archive_size_bytes > 0);

        let _ = fs::remove_file(&metadata.archive_path);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_rejects_nonexistent_directory() {
        let result = pack_snapshot(Path::new("/nonexistent/path/runfossil-test"));
        assert!(result.is_err());
    }

    #[test]
    fn pack_rejects_directory_without_manifest() {
        let dir = unique_test_dir("pack-no-manifest");
        fs::create_dir_all(&dir).expect("create dir");

        let result = pack_snapshot(&dir);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_preserves_raw_file_content() {
        let dir = unique_test_dir("pack-content");
        create_test_snapshot(&dir).expect("create test snapshot");

        let metadata = pack_snapshot(&dir).expect("pack snapshot");

        let compressed = fs::read(&metadata.archive_path).expect("read archive");
        let decompressed = zstd::decode_all(&compressed[..]).expect("decompress");

        let mut archive = tar::Archive::new(&decompressed[..]);
        let mut found_proc_stat = false;
        for entry_result in archive.entries().expect("read entries") {
            let mut entry = entry_result.expect("entry");
            if entry.path().expect("path") == Path::new("raw/proc/stat") {
                let mut contents = String::new();
                entry.read_to_string(&mut contents).expect("read entry");
                assert_eq!(contents, "cpu  0 0 0\n");
                found_proc_stat = true;
            }
        }
        assert!(found_proc_stat);

        let _ = fs::remove_file(&metadata.archive_path);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_archive_metadata_json_writes_file() {
        let dir = unique_test_dir("pack-meta");
        create_test_snapshot(&dir).expect("create test snapshot");

        let metadata = pack_snapshot(&dir).expect("pack snapshot");
        write_archive_metadata(&metadata).expect("write metadata");

        let meta_path = metadata.archive_path.with_extension("meta.json");
        assert!(meta_path.is_file());
        let meta_content = fs::read_to_string(&meta_path).expect("read meta");
        assert!(meta_content.contains("\"archive_sha256\""));
        assert!(meta_content.contains("\"snapshot_was_complete\": true"));

        let _ = fs::remove_file(&metadata.archive_path);
        let _ = fs::remove_file(&meta_path);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_partial_snapshot_reports_not_complete() {
        let dir = unique_test_dir("pack-partial");
        create_test_snapshot(&dir).expect("create test snapshot");
        fs::remove_file(dir.join(COMPLETE_MARKER)).expect("remove marker");

        let metadata = pack_snapshot(&dir).expect("pack snapshot");
        assert!(!metadata.snapshot_was_complete);

        let _ = fs::remove_file(&metadata.archive_path);
        let _ = fs::remove_dir_all(&dir);
    }
}
