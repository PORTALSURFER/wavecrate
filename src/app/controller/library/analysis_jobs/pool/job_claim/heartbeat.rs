use crate::app::controller::library::analysis_jobs::db as analysis_db;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::db::{AnalysisConnections, open_connection_with_retry};

struct DecodeHeartbeatState {
    counter: u64,
    closed: bool,
    jobs_by_source: HashMap<PathBuf, HashSet<i64>>,
}

pub(crate) struct DecodeHeartbeatTracker {
    interval: Duration,
    state: Mutex<DecodeHeartbeatState>,
    ready: Condvar,
}

impl DecodeHeartbeatTracker {
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            interval,
            state: Mutex::new(DecodeHeartbeatState {
                counter: 0,
                closed: false,
                jobs_by_source: HashMap::new(),
            }),
            ready: Condvar::new(),
        }
    }

    pub(crate) fn register(&self, source_root: &Path, job_id: i64) {
        let mut state = self
            .state
            .lock()
            .expect("decode heartbeat tracker poisoned");
        let inserted = state
            .jobs_by_source
            .entry(source_root.to_path_buf())
            .or_default()
            .insert(job_id);
        if inserted {
            state.counter = state.counter.wrapping_add(1);
            self.ready.notify_one();
        }
    }

    pub(crate) fn unregister(&self, source_root: &Path, job_id: i64) {
        let mut state = self
            .state
            .lock()
            .expect("decode heartbeat tracker poisoned");
        let mut changed = false;
        if let Some(job_ids) = state.jobs_by_source.get_mut(source_root) {
            changed = job_ids.remove(&job_id);
            if job_ids.is_empty() {
                state.jobs_by_source.remove(source_root);
            }
        }
        if changed {
            state.counter = state.counter.wrapping_add(1);
            self.ready.notify_one();
        }
    }

    pub(crate) fn close(&self) {
        let mut state = self
            .state
            .lock()
            .expect("decode heartbeat tracker poisoned");
        state.closed = true;
        state.counter = state.counter.wrapping_add(1);
        self.ready.notify_one();
    }

    pub(crate) fn clear(&self) {
        let mut state = self
            .state
            .lock()
            .expect("decode heartbeat tracker poisoned");
        if state.jobs_by_source.is_empty() {
            return;
        }
        state.jobs_by_source.clear();
        state.counter = state.counter.wrapping_add(1);
        self.ready.notify_one();
    }

    fn snapshot(&self) -> (bool, HashMap<PathBuf, Vec<i64>>) {
        let state = self
            .state
            .lock()
            .expect("decode heartbeat tracker poisoned");
        let jobs = state
            .jobs_by_source
            .iter()
            .map(|(source_root, job_ids)| {
                (
                    source_root.clone(),
                    job_ids.iter().copied().collect::<Vec<_>>(),
                )
            })
            .collect();
        (state.closed, jobs)
    }

    fn wait_for_change(&self, seen: &mut u64, timeout: Option<Duration>) -> bool {
        let state = self
            .state
            .lock()
            .expect("decode heartbeat tracker poisoned");
        if state.counter != *seen {
            *seen = state.counter;
            return true;
        }
        let state = if let Some(timeout) = timeout {
            let (state, _) = self
                .ready
                .wait_timeout(state, timeout)
                .expect("decode heartbeat tracker poisoned");
            state
        } else {
            self.ready
                .wait(state)
                .expect("decode heartbeat tracker poisoned")
        };
        if state.counter != *seen {
            *seen = state.counter;
            return true;
        }
        false
    }
}

pub(crate) fn spawn_decode_heartbeat_worker(
    tracker: Arc<DecodeHeartbeatTracker>,
) -> JoinHandle<()> {
    std::thread::spawn(move || run_decode_heartbeat_worker(tracker))
}

fn run_decode_heartbeat_worker(tracker: Arc<DecodeHeartbeatTracker>) {
    let mut connections = AnalysisConnections::new();
    let mut seen_counter = 0u64;
    let mut last_touch = Instant::now() - tracker.interval;
    loop {
        let (closed, jobs_by_source) = tracker.snapshot();
        if closed {
            break;
        }
        if jobs_by_source.is_empty() {
            let _ = tracker.wait_for_change(&mut seen_counter, None);
            continue;
        }
        let elapsed = last_touch.elapsed();
        if elapsed < tracker.interval {
            let _ = tracker.wait_for_change(&mut seen_counter, Some(tracker.interval - elapsed));
            continue;
        }
        for (source_root, job_ids) in jobs_by_source {
            if job_ids.is_empty() {
                continue;
            }
            match open_connection_with_retry(&mut connections, &source_root) {
                Ok(conn) => {
                    let _ = analysis_db::touch_running_at(conn, &job_ids);
                }
                Err(err) => {
                    tracing::warn!(
                        source_root = %source_root.display(),
                        error = %err,
                        "Analysis decode heartbeat failed to open DB"
                    );
                }
            }
        }
        last_touch = Instant::now();
    }
}
