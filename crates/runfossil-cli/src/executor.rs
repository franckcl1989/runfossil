#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
#![doc = "Bounded concurrent executor for runfossil capture tasks."]

use std::fmt;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use runfossil_store::StoreError;

#[derive(Clone, Debug)]
pub(crate) struct CaptureBudget {
    pub max_concurrency: NonZeroUsize,
    pub global_timeout: Duration,
    pub max_total_bytes: u64,
    pub per_task_timeout: Duration,
}

impl Default for CaptureBudget {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(1);
        let concurrency = match NonZeroUsize::new(cpus.clamp(2, 8)) {
            Some(n) => n,
            None => NonZeroUsize::MIN,
        };
        Self {
            max_concurrency: concurrency,
            global_timeout: Duration::from_secs(30),
            max_total_bytes: 512 * 1024 * 1024,
            per_task_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Clone)]
pub(crate) struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CancelToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

pub(crate) type CollectorFn = fn(&mut runfossil_store::SnapshotStore) -> Result<(), StoreError>;

#[derive(Clone)]
pub(crate) struct CaptureTask {
    pub name: &'static str,
    pub collector: CollectorFn,
    pub priority: u8,
}

#[derive(Debug)]
pub(crate) enum ExecutorError {
    TaskFailed {
        name: &'static str,
        source: StoreError,
    },
    GlobalTimeout,
    ByteBudgetExceeded {
        bytes_written: u64,
        max_bytes: u64,
    },
    Cancelled,
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskFailed { name, source } => write!(f, "collector '{name}' failed: {source}"),
            Self::GlobalTimeout => f.write_str("global capture timeout exceeded"),
            Self::ByteBudgetExceeded {
                bytes_written,
                max_bytes,
            } => write!(
                f,
                "byte budget exceeded: {bytes_written} bytes written (limit {max_bytes})",
            ),
            Self::Cancelled => f.write_str("capture cancelled"),
        }
    }
}

impl std::error::Error for ExecutorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TaskFailed { source, .. } => Some(source),
            Self::GlobalTimeout | Self::ByteBudgetExceeded { .. } | Self::Cancelled => None,
        }
    }
}

/// Runs capture tasks in priority order with bounded concurrency.
///
/// Each task is given [`CaptureBudget::per_task_timeout`] to complete. Tasks
/// that exceed their per-task deadline are recorded as failures but do not
/// prevent other tasks from running. The global timeout and byte budget act
/// as additional safety valves.
pub(crate) fn run_capture_tasks(
    store: Arc<Mutex<runfossil_store::SnapshotStore>>,
    tasks: Vec<CaptureTask>,
    budget: &CaptureBudget,
    cancel: CancelToken,
    quiet: bool,
) -> Result<(), ExecutorError> {
    if tasks.is_empty() {
        return Ok(());
    }

    let capture_start = Instant::now();
    let mut all_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    let mut sorted_tasks = tasks;
    sorted_tasks.sort_by_key(|t| t.priority);

    let batch_size = budget.max_concurrency.get();
    let num_batches = sorted_tasks.len().div_ceil(batch_size);
    let first_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    for batch_idx in 0..num_batches {
        if cancel.is_cancelled() {
            join_remaining(&mut all_handles);
            return Err(ExecutorError::Cancelled);
        }

        if capture_start.elapsed() > budget.global_timeout {
            join_remaining(&mut all_handles);
            return Err(ExecutorError::GlobalTimeout);
        }

        {
            let Ok(store_guard) = store.lock() else {
                join_remaining(&mut all_handles);
                return Err(ExecutorError::TaskFailed {
                    name: "capture",
                    source: StoreError::io(
                        "check byte budget",
                        std::io::Error::other("store mutex poisoned"),
                    ),
                });
            };
            if store_guard.total_bytes_written() >= budget.max_total_bytes {
                join_remaining(&mut all_handles);
                return Err(ExecutorError::ByteBudgetExceeded {
                    bytes_written: store_guard.total_bytes_written(),
                    max_bytes: budget.max_total_bytes,
                });
            }
        }

        let start = batch_idx * batch_size;
        let end = (start + batch_size).min(sorted_tasks.len());
        let batch = &sorted_tasks[start..end];

        if !quiet {
            for task in batch {
                eprintln!("Collecting {}...", task.name);
            }
        }

        struct ChanPair {
            handle: std::thread::JoinHandle<()>,
            rx: mpsc::Receiver<Result<(), String>>,
            task_name: &'static str,
        }

        let mut channels: Vec<ChanPair> = Vec::new();

        for task in batch {
            if cancel.is_cancelled() {
                break;
            }

            {
                let Ok(guard) = first_error.lock() else {
                    break;
                };
                if guard.is_some() {
                    break;
                }
            }

            let store = Arc::clone(&store);
            let cancel = cancel.clone();
            let task = task.clone();
            let (tx, rx) = mpsc::channel::<Result<(), String>>();

            let task_name = task.name;
            match std::thread::Builder::new()
                .name(format!("runfossil-{task_name}"))
                .spawn(move || {
                    if cancel.is_cancelled() {
                        let _ = tx.send(Ok(()));
                        return;
                    }

                    let result = match store.lock() {
                        Ok(mut store_guard) => (task.collector)(&mut store_guard),
                        Err(_) => {
                            let _ = tx.send(Err(format!(
                                "collector '{task_name}': store mutex poisoned"
                            )));
                            return;
                        }
                    };

                    match result {
                        Ok(()) => {
                            let _ = tx.send(Ok(()));
                        }
                        Err(source) => {
                            let _ = tx.send(Err(format!("collector '{task_name}': {source}")));
                        }
                    }
                }) {
                Ok(handle) => channels.push(ChanPair {
                    handle,
                    rx,
                    task_name,
                }),
                Err(source) => {
                    if let Ok(mut err) = first_error.lock()
                        && err.is_none()
                    {
                        *err = Some(format!(
                            "failed to spawn thread for '{task_name}': {source}"
                        ));
                    }
                }
            }
        }

        for chan in channels {
            let ChanPair {
                handle,
                rx,
                task_name,
            } = chan;

            if capture_start.elapsed() > budget.global_timeout {
                drop(rx);
                if let Ok(mut err) = first_error.lock()
                    && err.is_none()
                {
                    *err = Some(format!(
                        "collector '{task_name}': global capture timeout exceeded",
                    ));
                }
                all_handles.push(handle);
                continue;
            }

            let timed_out = match rx.recv_timeout(budget.per_task_timeout) {
                Ok(Ok(())) => false,
                Ok(Err(msg)) => {
                    if let Ok(mut err) = first_error.lock()
                        && err.is_none()
                    {
                        *err = Some(msg);
                    }
                    false
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    drop(rx);
                    if let Ok(mut err) = first_error.lock()
                        && err.is_none()
                    {
                        *err = Some(format!(
                            "collector '{task_name}': exceeded per-task timeout",
                        ));
                    }
                    true
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if let Ok(mut err) = first_error.lock()
                        && err.is_none()
                    {
                        *err = Some(format!(
                            "collector '{task_name}': thread panicked before sending result",
                        ));
                    }
                    true
                }
            };

            let join_limit = if timed_out {
                Duration::from_secs(5)
            } else {
                Duration::from_secs(2)
            };
            let join_deadline = Instant::now() + join_limit;
            while !handle.is_finished() && Instant::now() < join_deadline {
                std::thread::sleep(Duration::from_millis(100));
            }
            all_handles.push(handle);
        }

        if cancel.is_cancelled() {
            join_remaining(&mut all_handles);
            return Err(ExecutorError::Cancelled);
        }

        if let Ok(guard) = first_error.lock() {
            if guard.is_some() {
                break;
            }
        } else {
            join_remaining(&mut all_handles);
            return Err(ExecutorError::TaskFailed {
                name: "capture",
                source: StoreError::io(
                    "read error state",
                    std::io::Error::other("error mutex poisoned"),
                ),
            });
        }
    }

    join_remaining(&mut all_handles);

    if let Ok(guard) = first_error.lock() {
        if let Some(ref msg) = *guard {
            Err(ExecutorError::TaskFailed {
                name: "capture",
                source: StoreError::io("collector task failed", std::io::Error::other(msg.clone())),
            })
        } else {
            Ok(())
        }
    } else {
        Err(ExecutorError::TaskFailed {
            name: "capture",
            source: StoreError::io(
                "read error state",
                std::io::Error::other("error mutex poisoned"),
            ),
        })
    }
}

fn join_remaining(handles: &mut Vec<std::thread::JoinHandle<()>>) {
    let deadline = Instant::now() + Duration::from_secs(3);
    for handle in handles.drain(..) {
        while !handle.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use runfossil_core::EffectiveUid;
    use runfossil_store::{HostMetadata, SnapshotMetadata, SnapshotStore, StoreError};

    use super::*;

    fn create_test_store() -> SnapshotStore {
        let dir =
            tempfile::TempDir::with_prefix("runfossil-executor-test").expect("create temp dir");
        let snapshot_dir = dir.path().join("snapshot");
        let metadata = SnapshotMetadata {
            tool_version: "0.0.0".to_string(),
            started_at_unix_ns: "0".to_string(),
            effective_uid: EffectiveUid::ROOT,
            host: HostMetadata {
                hostname: "test".into(),
                boot_id: "00000000-0000-0000-0000-000000000000".into(),
                kernel_release: "test".into(),
                machine: "x86_64".into(),
            },
        };
        SnapshotStore::create(&snapshot_dir, metadata).expect("create test store")
    }

    fn noop_collector(_store: &mut SnapshotStore) -> Result<(), StoreError> {
        Ok(())
    }

    fn failing_collector(_store: &mut SnapshotStore) -> Result<(), StoreError> {
        Err(StoreError::io(
            "test failure",
            std::io::Error::other("test"),
        ))
    }

    #[test]
    fn executor_runs_empty_task_list() {
        let store = create_test_store();
        let cancel = CancelToken::new();
        let budget = CaptureBudget::default();
        let store = Arc::new(std::sync::Mutex::new(store));
        let result = run_capture_tasks(store, vec![], &budget, cancel, false);
        assert!(result.is_ok());
    }

    #[test]
    fn executor_runs_single_task() {
        let store = create_test_store();
        let cancel = CancelToken::new();
        let budget = CaptureBudget::default();
        let store = Arc::new(std::sync::Mutex::new(store));
        let tasks = vec![CaptureTask {
            name: "test",
            collector: noop_collector,
            priority: 0,
        }];
        let result = run_capture_tasks(store, tasks, &budget, cancel, false);
        assert!(result.is_ok());
    }

    #[test]
    fn executor_fails_on_collector_error() {
        let store = create_test_store();
        let cancel = CancelToken::new();
        let budget = CaptureBudget::default();
        let store = Arc::new(std::sync::Mutex::new(store));
        let tasks = vec![CaptureTask {
            name: "failing",
            collector: failing_collector,
            priority: 0,
        }];
        let result = run_capture_tasks(store, tasks, &budget, cancel, false);
        assert!(result.is_err());
    }

    #[test]
    fn executor_cancels_on_token() {
        let store = create_test_store();
        let cancel = CancelToken::new();
        cancel.cancel();
        let budget = CaptureBudget::default();
        let store = Arc::new(std::sync::Mutex::new(store));
        let tasks = vec![CaptureTask {
            name: "test",
            collector: noop_collector,
            priority: 0,
        }];
        let result = run_capture_tasks(store, tasks, &budget, cancel, false);
        assert!(matches!(result, Err(ExecutorError::Cancelled)));
    }

    #[test]
    fn cancel_token_default_is_not_cancelled() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn cancel_token_cancel_sets_flag() {
        let token = CancelToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn capture_budget_default_has_reasonable_concurrency() {
        let budget = CaptureBudget::default();
        assert!(budget.max_concurrency.get() >= 2);
        assert!(budget.max_concurrency.get() <= 8);
    }

    #[test]
    fn executor_runs_multiple_tasks() {
        let store = create_test_store();
        let cancel = CancelToken::new();
        let budget = CaptureBudget::default();
        let store = Arc::new(std::sync::Mutex::new(store));
        let tasks: Vec<_> = (0..10)
            .map(|i| CaptureTask {
                name: "task",
                collector: noop_collector,
                priority: i,
            })
            .collect();
        let result = run_capture_tasks(store, tasks, &budget, cancel, false);
        assert!(result.is_ok());
    }
}
