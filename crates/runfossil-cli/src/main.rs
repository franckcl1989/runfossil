#![forbid(unsafe_code)]
#![doc = "Command-line entry point for runfossil."]

use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use nix::sys::signal::{SigSet, SigmaskHow, Signal, pthread_sigmask};
use runfossil_core::{EffectiveUid, PlanDecision, Priority, SourceSlug};
use runfossil_store::{HostMetadata, SnapshotMetadata, SnapshotStore, StoreError};

mod executor;

use executor::{CancelToken, CaptureBudget, CaptureTask, ExecutorError};

const HELP: &str = "\
runfossil — Linux runtime snapshot tool for incident forensics.

USAGE:
    runfossil capture [-o <output-dir>] [-q]
    runfossil pack <snapshot-dir>
    runfossil inspect <snapshot-dir>
    runfossil --version
    runfossil --help

FLAGS:
    -V, --version    Print version and exit.
    -h, --help       Print this help and exit.

CAPTURE OPTIONS:
    -o, --output <path>   Write snapshot to <path> instead of the current directory.
    -q, --quiet           Suppress progress output; print only the snapshot path.
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
        Some("-V" | "--version") => {
            println!("runfossil {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("capture") => {
            let (output_dir, quiet) = capture_opts(&mut args)?;
            ensure_no_extra_arg(&mut args)?;
            run_capture(output_dir, quiet)
        }
        Some("pack") => {
            let snapshot_dir = required_arg(&mut args, "snapshot-dir")?;
            ensure_no_extra_arg(&mut args)?;
            run_pack(Path::new(&snapshot_dir))
        }
        Some("inspect") => {
            let snapshot_dir = required_arg(&mut args, "snapshot-dir")?;
            ensure_no_extra_arg(&mut args)?;
            run_inspect(Path::new(&snapshot_dir))
        }
        Some(other) => Err(CliError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

fn capture_opts(
    args: &mut impl Iterator<Item = String>,
) -> Result<(Option<PathBuf>, bool), CliError> {
    let mut output_dir: Option<PathBuf> = None;
    let mut quiet = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                let path = required_arg(args, "output-dir")?;
                output_dir = Some(PathBuf::from(path));
            }
            "-q" | "--quiet" => {
                quiet = true;
            }
            _ => {
                // put back for ensure_no_extra_arg
                return Err(CliError::UnexpectedArgument { argument: arg });
            }
        }
    }
    Ok((output_dir, quiet))
}

fn run_pack(snapshot_dir: &Path) -> Result<(), CliError> {
    let metadata = runfossil_pack::pack_snapshot(snapshot_dir).map_err(CliError::Pack)?;
    runfossil_pack::write_archive_metadata(&metadata).map_err(CliError::Pack)?;
    println!("{}", metadata.archive_path.display());
    Ok(())
}

fn run_inspect(snapshot_dir: &Path) -> Result<(), CliError> {
    let report = runfossil_pack::inspect_snapshot(snapshot_dir).map_err(CliError::Inspector)?;
    print!("{report}");
    Ok(())
}

fn run_capture(output_dir: Option<PathBuf>, quiet: bool) -> Result<(), CliError> {
    let effective_uid = current_effective_uid()?;
    if !effective_uid.is_root() {
        return Err(CliError::NotRoot { effective_uid });
    }

    let started_at_unix_ns = unix_time_ns()?;
    let host = read_host_metadata()?;
    let snapshot_name = snapshot_name(&host);
    let parent_dir =
        output_dir.unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let snapshot_dir = parent_dir.join(&snapshot_name);

    let metadata = SnapshotMetadata {
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        started_at_unix_ns: started_at_unix_ns.to_string(),
        effective_uid,
        host,
    };

    let store = SnapshotStore::create(&snapshot_dir, metadata).map_err(CliError::Store)?;

    let probe = runfossil_plan::probe_host();
    let plan = runfossil_plan::build_plan(probe);
    store
        .write_raw_file(Path::new("plan.json"), plan.to_json().as_bytes())
        .map_err(CliError::Store)?;

    let cancel = CancelToken::new();
    install_signal_handler(cancel.clone(), snapshot_dir.clone());

    let store = Arc::new(store);
    let store_for_executor = Arc::clone(&store);
    let tasks = capture_task_list(&plan);
    let budget = CaptureBudget::default();

    let capture_start = std::time::Instant::now();
    let exec_result =
        executor::run_capture_tasks(store_for_executor, tasks, &budget, cancel, quiet);

    match exec_result {
        Ok(()) => {
            let finished_at_unix_ns = unix_time_ns()?;
            store
                .finalize(&finished_at_unix_ns.to_string())
                .map_err(CliError::Store)?;
            let elapsed = capture_start.elapsed();
            let entry_count = store.entry_count();
            if !quiet {
                eprintln!(
                    "Capture complete: {} objects in {:.1}s",
                    entry_count,
                    elapsed.as_secs_f64()
                );
            }
            println!("{}", snapshot_dir.display());
            Ok(())
        }
        Err(ExecutorError::Cancelled) => {
            let finished_at_unix_ns = unix_time_ns()?;
            store
                .close_partial(&finished_at_unix_ns.to_string())
                .map_err(CliError::Store)?;
            if !quiet {
                eprintln!(
                    "Capture cancelled; partial snapshot preserved at {}",
                    snapshot_dir.display()
                );
            }
            println!("{}", snapshot_dir.display());
            Ok(())
        }
        Err(ExecutorError::TaskFailed { name: _, source: _ }) => {
            let finished_at_unix_ns = unix_time_ns()?;
            store
                .close_partial(&finished_at_unix_ns.to_string())
                .map_err(CliError::Store)?;
            if !quiet {
                eprintln!(
                    "Capture completed with errors; partial snapshot at {}",
                    snapshot_dir.display()
                );
            }
            println!("{}", snapshot_dir.display());
            Ok(())
        }
        Err(ExecutorError::GlobalTimeout) => {
            let finished_at_unix_ns = unix_time_ns()?;
            store
                .close_partial(&finished_at_unix_ns.to_string())
                .map_err(CliError::Store)?;
            Err(CliError::CaptureTimeout)
        }
        Err(ExecutorError::ByteBudgetExceeded {
            bytes_written: _,
            max_bytes: _,
        }) => {
            let finished_at_unix_ns = unix_time_ns()?;
            store
                .close_partial(&finished_at_unix_ns.to_string())
                .map_err(CliError::Store)?;
            Err(CliError::ByteBudgetExceeded)
        }
    }
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

fn snapshot_name(host: &HostMetadata) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Format: YYYYMMDDTHHMMSSZ — matching the snapshot spec example.
    // This is a simplified conversion; it stays within the stdlib without pulling chrono.
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Compute year/month/day from days since epoch (civil date algorithm).
    let (year, month, day) = civil_from_days(days_since_epoch as i64 + 719_468);

    let hostname = sanitize_snapshot_component(&host.hostname);
    let boot_id_short =
        sanitize_snapshot_component(&host.boot_id.chars().take(8).collect::<String>());
    format!(
        "snapshot-{year:04}{month:02}{day:02}T{hours:02}{minutes:02}{seconds:02}Z-{hostname}-boot-{boot_id_short}"
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Howard Hinnant's algorithm: days since 0000-03-01.
    let z = days;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
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
    NotRoot {
        effective_uid: EffectiveUid,
    },
    MissingEffectiveUid,
    ParseUid {
        value: String,
        source: std::num::ParseIntError,
    },
    ClockBeforeEpoch,
    CaptureTimeout,
    ByteBudgetExceeded,
    Io {
        context: &'static str,
        source: std::io::Error,
    },
    Store(StoreError),
    Pack(std::io::Error),
    Inspector(runfossil_pack::InspectionError),
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
            Self::MissingEffectiveUid | Self::ParseUid { .. } | Self::ClockBeforeEpoch => 70,
            Self::CaptureTimeout
            | Self::ByteBudgetExceeded
            | Self::Io { .. }
            | Self::Store(_)
            | Self::Pack(_)
            | Self::Inspector(_) => 74,
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
            Self::CaptureTimeout => formatter.write_str("global capture timeout exceeded"),
            Self::ByteBudgetExceeded => formatter.write_str("capture byte budget exceeded"),
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
            Self::Store(source) => write!(formatter, "{source}"),
            Self::Pack(source) => write!(formatter, "packaging error: {source}"),
            Self::Inspector(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseUid { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
            Self::Store(source) => Some(source),
            Self::Pack(source) => Some(source),
            Self::Inspector(source) => Some(source),
            Self::UnknownCommand { .. }
            | Self::UnexpectedArgument { .. }
            | Self::MissingArgument { .. }
            | Self::NotRoot { .. }
            | Self::MissingEffectiveUid
            | Self::ClockBeforeEpoch
            | Self::CaptureTimeout
            | Self::ByteBudgetExceeded => None,
        }
    }
}

fn install_signal_handler(cancel: CancelToken, snapshot_dir: PathBuf) {
    let signals = capture_signal_set();
    let mut previous_mask = SigSet::empty();
    if let Err(source) = pthread_sigmask(
        SigmaskHow::SIG_BLOCK,
        Some(&signals),
        Some(&mut previous_mask),
    ) {
        eprintln!("warning: could not install signal cancellation handler: {source}");
        return;
    }

    if let Err(source) = std::thread::Builder::new()
        .name("runfossil-signal".into())
        .spawn(move || {
            let mut reported = false;
            loop {
                match signals.wait() {
                    Ok(signal) => {
                        cancel.cancel();
                        if !reported {
                            reported = true;
                            eprintln!(
                                "{signal:?} received; cancelling capture. Partial snapshot at {}",
                                snapshot_dir.display()
                            );
                        }
                    }
                    Err(source) => {
                        eprintln!("warning: signal wait failed: {source}");
                        break;
                    }
                }
            }
        })
    {
        let _ = pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&previous_mask), None);
        eprintln!("warning: could not start signal cancellation thread: {source}");
    }
}

fn capture_signal_set() -> SigSet {
    let mut signals = SigSet::empty();
    signals.add(Signal::SIGINT);
    signals.add(Signal::SIGTERM);
    signals
}

fn capture_task_list(plan: &runfossil_plan::CapturePlan) -> Vec<CaptureTask> {
    let mut tasks = Vec::with_capacity(12);

    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Proc],
        "proc",
        runfossil_proc::collect_proc,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Sys, SourceSlug::Hardware],
        "sys",
        runfossil_proc::collect_sys,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Dev],
        "dev",
        runfossil_proc::collect_dev,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Kernel],
        "kmsg",
        runfossil_proc::collect_kmsg,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Crash],
        "crash",
        runfossil_proc::collect_crash,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Security],
        "security",
        runfossil_proc::collect_security,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Run],
        "run",
        runfossil_proc::collect_run,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Netlink],
        "netlink",
        runfossil_net::collect_netlink,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Logs],
        "logs",
        runfossil_service::collect_logs,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Time],
        "time",
        runfossil_service::collect_time,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Service, SourceSlug::Sessions],
        "service",
        runfossil_service::collect_service,
    );
    push_source_task(
        &mut tasks,
        plan,
        &[SourceSlug::Scheduler],
        "scheduler",
        runfossil_proc::collect_scheduler,
    );

    tasks
}

fn push_source_task(
    tasks: &mut Vec<CaptureTask>,
    plan: &runfossil_plan::CapturePlan,
    sources: &[SourceSlug],
    name: &'static str,
    collector: executor::CollectorFn,
) {
    if let Some(priority) = selected_source_priority(plan, sources) {
        tasks.push(CaptureTask {
            name,
            collector,
            priority,
        });
    }
}

fn selected_source_priority(
    plan: &runfossil_plan::CapturePlan,
    sources: &[SourceSlug],
) -> Option<u8> {
    plan.tasks()
        .iter()
        .filter(|task| sources.contains(&task.source) && should_execute(task.decision))
        .map(|task| priority_rank(task.priority))
        .min()
}

const fn should_execute(decision: PlanDecision) -> bool {
    matches!(decision, PlanDecision::Scheduled | PlanDecision::Limited)
}

const fn priority_rank(priority: Priority) -> u8 {
    match priority {
        Priority::P0 => 0,
        Priority::P1 => 1,
        Priority::P2 => 2,
        Priority::P3 => 3,
        Priority::P4 => 4,
        Priority::NotApplicable => u8::MAX,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use runfossil_core::CoverageDecision;
    use runfossil_plan::{
        CapturePlan, CapturePlanProbes, PlannedTask, PressureLevels, RiskLevel, TaskLimits,
    };

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
        let name = snapshot_name(&host);
        assert!(name.starts_with("snapshot-"));
        assert!(name.contains("-prod-db-01-boot-12345678"));
        // UTC time component: YYYYMMDDTHHMMSSZ
        let after_prefix = name
            .strip_prefix("snapshot-")
            .expect("snapshot name must start with 'snapshot-'");
        let time_part = after_prefix
            .split('-')
            .next()
            .expect("time component must be present");
        assert_eq!(time_part.len(), 16); // 8 digits + T + 6 digits + Z
        assert!(time_part.contains('T'));
        assert!(time_part.ends_with('Z'));
    }

    #[test]
    fn capture_task_list_uses_plan_decisions() {
        let mut plan = CapturePlan::new(
            "test",
            CapturePlanProbes::empty(),
            PressureLevels::default(),
        );
        plan.push(planned_task(
            "proc.meminfo",
            SourceSlug::Proc,
            PlanDecision::SkippedByPolicy,
            Priority::P0,
        ));
        plan.push(planned_task(
            "logs.syslog",
            SourceSlug::Logs,
            PlanDecision::Scheduled,
            Priority::P3,
        ));

        let tasks = capture_task_list(&plan);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "logs");
        assert_eq!(tasks[0].priority, 3);
    }

    fn planned_task(
        id: &str,
        source: SourceSlug,
        decision: PlanDecision,
        priority: Priority,
    ) -> PlannedTask {
        PlannedTask {
            id: id.to_string(),
            source,
            domain: "test".to_string(),
            object: "object".to_string(),
            coverage_decision: CoverageDecision::Collect,
            decision,
            priority,
            risk: RiskLevel::Low,
            reason: "test".to_string(),
            limits: TaskLimits::new(1, 1, 1, 0),
            dependencies: Vec::new(),
        }
    }
}
