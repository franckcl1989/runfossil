#![forbid(unsafe_code)]
#![doc = "Command-line entry point for runfossil."]

use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use runfossil_core::EffectiveUid;
use runfossil_store::{HostMetadata, SnapshotMetadata, SnapshotStore, StoreError};

const HELP: &str = "\
runfossil

USAGE:
    runfossil capture
    runfossil pack <snapshot-dir>
    runfossil inspect <snapshot-dir>
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run() -> Result<(), CliError> {
    let mut args = env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        None | Some("-h" | "--help") => {
            print!("{HELP}");
            Ok(())
        }
        Some("capture") => {
            ensure_no_extra_arg(&mut args)?;
            run_capture()
        }
        Some("pack") => {
            let snapshot_dir = required_arg(&mut args, "snapshot-dir")?;
            ensure_no_extra_arg(&mut args)?;
            Err(CliError::NotYetImplemented {
                command: "pack",
                detail: snapshot_dir,
            })
        }
        Some("inspect") => {
            let snapshot_dir = required_arg(&mut args, "snapshot-dir")?;
            ensure_no_extra_arg(&mut args)?;
            Err(CliError::NotYetImplemented {
                command: "inspect",
                detail: snapshot_dir,
            })
        }
        Some(other) => Err(CliError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

fn run_capture() -> Result<(), CliError> {
    let effective_uid = current_effective_uid()?;
    if !effective_uid.is_root() {
        return Err(CliError::NotRoot { effective_uid });
    }

    let started_at_unix_ns = unix_time_ns()?;
    let host = read_host_metadata()?;
    let snapshot_name = snapshot_name(started_at_unix_ns, &host);
    let snapshot_dir = env::current_dir()
        .map_err(|source| CliError::io("read current directory", source))?
        .join(snapshot_name);

    let metadata = SnapshotMetadata {
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        started_at_unix_ns: started_at_unix_ns.to_string(),
        effective_uid,
        host,
    };

    let mut store = SnapshotStore::create(&snapshot_dir, metadata).map_err(CliError::Store)?;

    let probe = runfossil_plan::probe_host();
    let plan = runfossil_plan::build_plan(probe);
    store
        .write_raw_file(Path::new("plan.json"), plan.to_json().as_bytes())
        .map_err(CliError::Store)?;

    runfossil_proc::collect_proc(&mut store).map_err(CliError::Store)?;
    runfossil_proc::collect_sys(&mut store).map_err(CliError::Store)?;

    let finished_at_unix_ns = unix_time_ns()?;
    store
        .finalize(&finished_at_unix_ns.to_string())
        .map_err(CliError::Store)?;
    println!("{}", snapshot_dir.display());
    Ok(())
}

fn ensure_no_extra_arg(args: &mut impl Iterator<Item = String>) -> Result<(), CliError> {
    match args.next() {
        Some(argument) => Err(CliError::UnexpectedArgument { argument }),
        None => Ok(()),
    }
}

fn required_arg(
    args: &mut impl Iterator<Item = String>,
    name: &'static str,
) -> Result<String, CliError> {
    args.next().ok_or(CliError::MissingArgument { name })
}

fn current_effective_uid() -> Result<EffectiveUid, CliError> {
    let status = read_trimmed("/proc/self/status", "read /proc/self/status")?;
    parse_effective_uid(&status)
}

fn parse_effective_uid(status: &str) -> Result<EffectiveUid, CliError> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let mut fields = rest.split_whitespace();
            let _real_uid = fields.next();
            let effective_uid = fields.next().ok_or(CliError::MissingEffectiveUid)?;
            let parsed = effective_uid
                .parse::<u32>()
                .map_err(|source| CliError::ParseUid {
                    value: effective_uid.to_string(),
                    source,
                })?;
            return Ok(EffectiveUid::new(parsed));
        }
    }

    Err(CliError::MissingEffectiveUid)
}

fn read_host_metadata() -> Result<HostMetadata, CliError> {
    Ok(HostMetadata {
        hostname: read_trimmed("/proc/sys/kernel/hostname", "read hostname")?,
        boot_id: read_trimmed("/proc/sys/kernel/random/boot_id", "read boot id")?,
        kernel_release: read_trimmed("/proc/sys/kernel/osrelease", "read kernel release")?,
        machine: env::consts::ARCH.to_string(),
    })
}

fn read_trimmed(path: impl AsRef<Path>, context: &'static str) -> Result<String, CliError> {
    let contents = fs::read_to_string(path).map_err(|source| CliError::io(context, source))?;
    Ok(contents.trim().to_string())
}

fn unix_time_ns() -> Result<u128, CliError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|_| CliError::ClockBeforeEpoch)
}

fn snapshot_name(started_at_unix_ns: u128, host: &HostMetadata) -> String {
    let hostname = sanitize_snapshot_component(&host.hostname);
    let boot_id_short =
        sanitize_snapshot_component(&host.boot_id.chars().take(8).collect::<String>());
    format!("snapshot-{started_at_unix_ns}-{hostname}-boot-{boot_id_short}")
}

fn sanitize_snapshot_component(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            output.push(character);
        } else {
            output.push('-');
        }
    }

    if output.is_empty() {
        "unknown".to_string()
    } else {
        output
    }
}

#[derive(Debug)]
enum CliError {
    UnknownCommand {
        command: String,
    },
    UnexpectedArgument {
        argument: String,
    },
    MissingArgument {
        name: &'static str,
    },
    NotYetImplemented {
        command: &'static str,
        detail: String,
    },
    NotRoot {
        effective_uid: EffectiveUid,
    },
    MissingEffectiveUid,
    ParseUid {
        value: String,
        source: std::num::ParseIntError,
    },
    ClockBeforeEpoch,
    Io {
        context: &'static str,
        source: std::io::Error,
    },
    Store(StoreError),
}

impl CliError {
    fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }

    const fn exit_code(&self) -> u8 {
        match self {
            Self::UnknownCommand { .. }
            | Self::UnexpectedArgument { .. }
            | Self::MissingArgument { .. } => 64,
            Self::NotRoot { .. } => 77,
            Self::NotYetImplemented { .. } => 69,
            Self::MissingEffectiveUid | Self::ParseUid { .. } | Self::ClockBeforeEpoch => 70,
            Self::Io { .. } | Self::Store(_) => 74,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand { command } => {
                write!(formatter, "unknown command '{command}'\n{HELP}")
            }
            Self::UnexpectedArgument { argument } => {
                write!(formatter, "unexpected argument '{argument}'\n{HELP}")
            }
            Self::MissingArgument { name } => {
                write!(formatter, "missing argument <{name}>\n{HELP}")
            }
            Self::NotYetImplemented { command, detail } => {
                write!(
                    formatter,
                    "'{command}' is not implemented yet for '{detail}'"
                )
            }
            Self::NotRoot { effective_uid } => {
                write!(
                    formatter,
                    "runfossil capture requires effective UID 0; current effective UID is {effective_uid}"
                )
            }
            Self::MissingEffectiveUid => {
                formatter.write_str("could not find effective UID in /proc/self/status")
            }
            Self::ParseUid { value, source } => {
                write!(
                    formatter,
                    "could not parse effective UID '{value}': {source}"
                )
            }
            Self::ClockBeforeEpoch => formatter.write_str("system clock is before the Unix epoch"),
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
            Self::Store(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseUid { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
            Self::Store(source) => Some(source),
            Self::UnknownCommand { .. }
            | Self::UnexpectedArgument { .. }
            | Self::MissingArgument { .. }
            | Self::NotYetImplemented { .. }
            | Self::NotRoot { .. }
            | Self::MissingEffectiveUid
            | Self::ClockBeforeEpoch => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_effective_uid_from_proc_status() {
        let uid = parse_effective_uid("Name:\trunfossil\nUid:\t1000\t0\t1000\t1000\n");
        assert!(matches!(uid, Ok(uid) if uid == EffectiveUid::ROOT));
    }

    #[test]
    fn rejects_missing_effective_uid() {
        assert!(matches!(
            parse_effective_uid("Name:\trunfossil\n"),
            Err(CliError::MissingEffectiveUid)
        ));
    }

    #[test]
    fn snapshot_name_uses_safe_components() {
        let host = HostMetadata {
            hostname: "prod/db 01".to_string(),
            boot_id: "12345678-90ab".to_string(),
            kernel_release: "kernel".to_string(),
            machine: "x86_64".to_string(),
        };
        assert_eq!(
            snapshot_name(42, &host),
            "snapshot-42-prod-db-01-boot-12345678"
        );
    }
}
