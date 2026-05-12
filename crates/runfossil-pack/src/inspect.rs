#![forbid(unsafe_code)]
#![doc = "Snapshot inspection: offline validation and summary of partial or complete snapshots."]

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use runfossil_store::{COMPLETE_MARKER, ERRORS_FILE, MANIFEST_FILE, PLAN_FILE};

/// Summary of a snapshot inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectionReport {
    /// Path to the inspected snapshot directory.
    pub snapshot_dir: PathBuf,
    /// Whether `CAPTURE_COMPLETE` is present.
    pub is_complete: bool,
    /// Whether `manifest.json` exists and is readable.
    pub has_manifest: bool,
    /// Whether `plan.json` exists and is readable.
    pub has_plan: bool,
    /// Number of object entries found in the manifest.
    pub manifest_entry_count: usize,
    /// Counts of each manifest status value (e.g. captured, not_found).
    pub status_counts: BTreeMap<String, usize>,
    /// Number of lines in `errors.jsonl`.
    pub error_log_count: usize,
    /// Approximate count of files under `raw/`.
    pub raw_file_count: usize,
}

impl fmt::Display for InspectionReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "Snapshot: {}", self.snapshot_dir.display())?;
        writeln!(
            formatter,
            "  Complete:    {}",
            if self.is_complete {
                "yes"
            } else {
                "no (partial)"
            }
        )?;
        writeln!(
            formatter,
            "  Manifest:    {}",
            if self.has_manifest {
                format!("yes ({} entries)", self.manifest_entry_count)
            } else {
                "no".to_string()
            }
        )?;
        writeln!(
            formatter,
            "  Plan:        {}",
            if self.has_plan { "yes" } else { "no" }
        )?;
        writeln!(formatter, "  Error log:   {} lines", self.error_log_count)?;
        writeln!(formatter, "  Raw files:   {}", self.raw_file_count)?;

        if !self.status_counts.is_empty() {
            writeln!(formatter, "  Statuses:")?;
            for (status, count) in &self.status_counts {
                writeln!(formatter, "    {status}: {count}")?;
            }
        }

        Ok(())
    }
}

/// Error returned by snapshot inspection.
#[derive(Debug)]
pub enum InspectionError {
    /// An I/O error occurred during inspection.
    Io(io::Error),
}

impl fmt::Display for InspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "inspection I/O error: {err}"),
        }
    }
}

impl std::error::Error for InspectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
        }
    }
}

/// Inspect a snapshot directory and produce a summary report.
///
/// Works on both complete and partial snapshots. No root privileges required.
///
/// # Errors
///
/// Returns `InspectionError` if the directory does not exist or cannot be read.
pub fn inspect_snapshot(snapshot_dir: &Path) -> Result<InspectionReport, InspectionError> {
    if !snapshot_dir.is_dir() {
        return Err(InspectionError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("snapshot directory not found: {}", snapshot_dir.display()),
        )));
    }

    let is_complete = snapshot_dir.join(COMPLETE_MARKER).is_file();
    let has_manifest = snapshot_dir.join(MANIFEST_FILE).is_file();
    let has_plan = snapshot_dir.join(PLAN_FILE).is_file();

    let (manifest_entry_count, status_counts) = if has_manifest {
        match fs::read_to_string(snapshot_dir.join(MANIFEST_FILE)) {
            Ok(content) => {
                let count = count_manifest_objects(&content);
                let statuses = extract_status_counts(&content);
                (count, statuses)
            }
            Err(_) => (0, BTreeMap::new()),
        }
    } else {
        (0, BTreeMap::new())
    };

    let error_log_count = if snapshot_dir.join(ERRORS_FILE).is_file() {
        fs::read_to_string(snapshot_dir.join(ERRORS_FILE))
            .map(|content| count_lines(&content))
            .unwrap_or(0)
    } else {
        0
    };

    let raw_dir = snapshot_dir.join("raw");
    let raw_file_count = if raw_dir.is_dir() {
        count_files_recursive(&raw_dir).unwrap_or(0)
    } else {
        0
    };

    Ok(InspectionReport {
        snapshot_dir: snapshot_dir.to_path_buf(),
        is_complete,
        has_manifest,
        has_plan,
        manifest_entry_count,
        status_counts,
        error_log_count,
        raw_file_count,
    })
}

/// Count objects in a manifest JSON "objects" array by counting brace pairs.
fn count_manifest_objects(content: &str) -> usize {
    let mut object_count = 0;
    let mut depth = 0;
    for ch in content.chars() {
        match ch {
            '{' => {
                depth += 1;
                if depth == 2 {
                    object_count += 1;
                }
            }
            '}' if depth > 0 => {
                depth -= 1;
            }
            _ => {}
        }
    }
    object_count
}

/// Extract status values from manifest JSON entries.
///
/// Looks for `"status": "value"` patterns inside objects (depth >= 1).
fn extract_status_counts(content: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    let mut depth = 0;

    for line in content.lines() {
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' if depth > 0 => {
                    depth -= 1;
                }
                _ => {}
            }
        }
        if depth >= 2
            && let Some(status) = extract_json_string_field(line, "status")
        {
            *counts.entry(status).or_insert(0) += 1;
        }
    }
    counts
}

/// Extract the value of a JSON string field from a line like `"key": "value"`.
fn extract_json_string_field(line: &str, field_name: &str) -> Option<String> {
    let pattern = format!("\"{field_name}\":");
    let after_key = line.split(&pattern).nth(1)?;
    let after_open = after_key.trim_start().strip_prefix('"')?;
    after_open.split('"').next().map(String::from)
}

fn count_lines(content: &str) -> usize {
    content.lines().count()
}

fn count_files_recursive(dir: &Path) -> io::Result<usize> {
    let mut count = 0;
    let read_dir = fs::read_dir(dir)?;
    for entry_result in read_dir {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                match entry.metadata() {
                    Ok(m) => {
                        if m.is_dir() {
                            if let Ok(sub) = count_files_recursive(&path) {
                                count += sub;
                            }
                        } else if m.is_file() {
                            count += 1;
                        }
                    }
                    Err(_) => continue,
                }
            }
            Err(_) => continue,
        }
    }
    Ok(count)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_nanos(),
            Err(_) => 0,
        };
        std::env::temp_dir().join(format!(
            "runfossil-inspect-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn create_test_snapshot(dir: &Path, complete: bool) -> io::Result<()> {
        fs::create_dir_all(dir.join("raw").join("proc"))?;
        fs::create_dir_all(dir.join("meta"))?;

        let manifest = concat!(
            "{\n  \"schema_version\": 1,\n  \"tool\": \"runfossil\",\n",
            "  \"objects\": [\n",
            "    {\n      \"id\": \"proc.system.stat\",\n",
            "      \"source\": \"proc\",\n      \"domain\": \"system\",\n",
            "      \"object\": \"/proc/stat\",\n      \"kind\": \"file\",\n",
            "      \"status\": \"captured\"\n    },\n",
            "    {\n      \"id\": \"proc.system.uptime\",\n",
            "      \"source\": \"proc\",\n      \"domain\": \"system\",\n",
            "      \"object\": \"/proc/uptime\",\n      \"kind\": \"file\",\n",
            "      \"status\": \"captured\"\n    },\n",
            "    {\n      \"id\": \"sys.missing.file\",\n",
            "      \"source\": \"sys\",\n      \"domain\": \"missing\",\n",
            "      \"object\": \"/sys/missing\",\n      \"kind\": \"file\",\n",
            "      \"status\": \"not_found\"\n    }\n",
            "  ]\n}\n"
        );
        fs::write(dir.join(MANIFEST_FILE), manifest)?;
        fs::write(dir.join(PLAN_FILE), "{\"tasks\": []}\n")?;
        fs::write(
            dir.join(ERRORS_FILE),
            "{\"error\":\"one\"}\n{\"error\":\"two\"}\n",
        )?;
        fs::write(dir.join("raw/proc/stat"), b"cpu 0\n")?;

        if complete {
            fs::write(dir.join(COMPLETE_MARKER), "")?;
        }

        Ok(())
    }

    #[test]
    fn inspect_complete_snapshot_reports_all_fields() {
        let dir = unique_test_dir("inspect-complete");
        create_test_snapshot(&dir, true).expect("create test snapshot");

        let report = inspect_snapshot(&dir).expect("inspect snapshot");
        assert!(report.is_complete);
        assert!(report.has_manifest);
        assert!(report.has_plan);
        assert_eq!(report.manifest_entry_count, 3);
        assert_eq!(report.error_log_count, 2);
        assert_eq!(report.raw_file_count, 1);

        let captured = report.status_counts.get("captured").copied().unwrap_or(0);
        let not_found = report.status_counts.get("not_found").copied().unwrap_or(0);
        assert_eq!(captured, 2);
        assert_eq!(not_found, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn inspect_partial_snapshot_reports_not_complete() {
        let dir = unique_test_dir("inspect-partial");
        create_test_snapshot(&dir, false).expect("create test snapshot");

        let report = inspect_snapshot(&dir).expect("inspect snapshot");
        assert!(!report.is_complete);
        assert_eq!(report.manifest_entry_count, 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn inspect_rejects_nonexistent_directory() {
        let result = inspect_snapshot(Path::new("/nonexistent/inspect_test"));
        assert!(result.is_err());
    }

    #[test]
    fn inspect_empty_directory_has_no_entries() {
        let dir = unique_test_dir("inspect-empty");
        fs::create_dir_all(&dir).expect("create dir");

        let report = inspect_snapshot(&dir).expect("inspect snapshot");
        assert!(!report.is_complete);
        assert!(!report.has_manifest);
        assert_eq!(report.manifest_entry_count, 0);
        assert!(report.status_counts.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn inspect_display_output_is_non_empty() {
        let dir = unique_test_dir("inspect-display");
        create_test_snapshot(&dir, true).expect("create test snapshot");

        let report = inspect_snapshot(&dir).expect("inspect snapshot");
        let output = format!("{report}");
        assert!(output.contains("Complete:    yes"));
        assert!(output.contains("captured: 2"));
        assert!(output.contains("not_found: 1"));

        let _ = fs::remove_dir_all(&dir);
    }
}
