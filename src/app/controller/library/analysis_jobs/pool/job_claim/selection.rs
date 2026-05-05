//! Claim selection helpers for analysis jobs.

use super::claim::{SourceClaimDb, claim_batch_size, refresh_sources};
use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::wakeup::ClaimWakeup;
use crate::app::controller::library::source_write_priority;
use crate::logging::{DbDebugEvent, emit_db_debug_event};
use crate::sample_sources::SourceId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Defines claim error log interval.
const CLAIM_ERROR_LOG_INTERVAL: Duration = Duration::from_secs(30);

/// A selection outcome from the claim pool.
pub(crate) enum ClaimSelection {
    /// A job is ready to process.
    Job(db::ClaimedJob),
    /// No sources are available to claim from.
    NoSources,
    /// Sources exist, but no work is ready yet.
    Idle,
}

/// Tracks claim sources and selects the next job to work on.
pub(crate) struct ClaimSelector {
    sources: Vec<SourceClaimDb>,
    last_refresh: Instant,
    next_source: usize,
    local_queue: VecDeque<db::ClaimedJob>,
    claim_batch: usize,
    reset_done: Arc<Mutex<HashSet<PathBuf>>>,
    last_source_count: usize,
    claim_error_logs: HashMap<SourceId, ClaimErrorLogState>,
    #[cfg(test)]
    refresh_count: usize,
}

pub(crate) type SharedClaimSelector = Arc<Mutex<ClaimSelector>>;

impl ClaimSelector {
    /// Creates a new claim selector for decoding workers.
    pub(crate) fn new(reset_done: Arc<Mutex<HashSet<PathBuf>>>) -> Self {
        Self {
            sources: Vec::new(),
            last_refresh: Instant::now() - super::claim::SOURCE_REFRESH_INTERVAL,
            next_source: 0,
            local_queue: VecDeque::new(),
            claim_batch: claim_batch_size(),
            reset_done,
            last_source_count: 0,
            claim_error_logs: HashMap::new(),
            #[cfg(test)]
            refresh_count: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_sources_for_tests(
        sources: Vec<SourceClaimDb>,
        claim_batch: usize,
        reset_done: Arc<Mutex<HashSet<PathBuf>>>,
    ) -> Self {
        Self {
            last_source_count: sources.len(),
            sources,
            last_refresh: Instant::now(),
            next_source: 0,
            local_queue: VecDeque::new(),
            claim_batch: claim_batch.max(1),
            reset_done,
            #[cfg(test)]
            refresh_count: 0,
            claim_error_logs: HashMap::new(),
        }
    }

    /// Selects the next job if one is available.
    pub(crate) fn select_next(
        &mut self,
        allowed_source_ids: Option<&HashSet<SourceId>>,
        claim_wakeup: &ClaimWakeup,
    ) -> ClaimSelection {
        if let Some(job) = self.pop_local_job_not_owned_by_file_op() {
            return ClaimSelection::Job(job);
        }
        self.refresh_sources_if_needed(allowed_source_ids);
        if self.sources.is_empty() {
            claim_wakeup.finish_probe(super::claim::SOURCE_REFRESH_INTERVAL);
            return ClaimSelection::NoSources;
        }
        if !self.fill_local_queue() {
            claim_wakeup.finish_probe(Duration::from_secs(1));
            return ClaimSelection::Idle;
        }
        self.pop_local(claim_wakeup)
    }

    fn refresh_sources_if_needed(&mut self, allowed_source_ids: Option<&HashSet<SourceId>>) {
        #[cfg(test)]
        let should_refresh = self.last_refresh.elapsed() >= super::claim::SOURCE_REFRESH_INTERVAL;
        refresh_sources(
            &mut self.sources,
            &mut self.last_refresh,
            &self.reset_done,
            allowed_source_ids,
        );
        #[cfg(test)]
        if should_refresh {
            self.refresh_count = self.refresh_count.saturating_add(1);
        }
        if self.sources.len() != self.last_source_count {
            self.last_source_count = self.sources.len();
            tracing::debug!(
                "Analysis claim sources refreshed: {} source(s) available",
                self.last_source_count
            );
        }
    }

    fn fill_local_queue(&mut self) -> bool {
        let source_count = self.sources.len();
        for _ in 0..source_count {
            let idx = self.next_source % source_count;
            self.next_source = self.next_source.wrapping_add(1);
            let source = &mut self.sources[idx];
            if source_write_priority::file_op_write_priority_active(&source.source.id) {
                continue;
            }
            let source_id = source.source.id.clone();
            let source_root = source.source.root.clone();
            let jobs = match db::claim_next_jobs(
                &mut source.conn,
                &source.source.root,
                self.claim_batch,
            ) {
                Ok(jobs) => {
                    self.claim_error_logs.remove(&source_id);
                    jobs
                }
                Err(err) => {
                    record_claim_error(&mut self.claim_error_logs, &source_id, &source_root, &err);
                    continue;
                }
            };
            if !jobs.is_empty() {
                self.local_queue.extend(jobs);
                return true;
            }
        }
        false
    }

    /// Return a buffered claim whose source is not currently owned by a file op.
    fn pop_local_job_not_owned_by_file_op(&mut self) -> Option<db::ClaimedJob> {
        let active_sources = source_write_priority::active_file_op_write_priority_sources();
        if active_sources.is_empty() {
            return self.local_queue.pop_front();
        }
        let len = self.local_queue.len();
        for _ in 0..len {
            let job = self.local_queue.pop_front()?;
            let active = job
                .sample_id
                .split_once("::")
                .is_some_and(|(source_id, _)| {
                    active_sources
                        .iter()
                        .any(|active| active.as_str() == source_id)
                });
            if active {
                self.local_queue.push_back(job);
                continue;
            }
            return Some(job);
        }
        None
    }

    pub(crate) fn has_local_jobs(&self) -> bool {
        !self.local_queue.is_empty()
    }

    fn pop_local(&mut self, claim_wakeup: &ClaimWakeup) -> ClaimSelection {
        match self.local_queue.pop_front() {
            Some(job) => {
                claim_wakeup.finish_probe(Duration::ZERO);
                ClaimSelection::Job(job)
            }
            None => ClaimSelection::Idle,
        }
    }

    #[cfg(test)]
    pub(crate) fn refresh_count(&self) -> usize {
        self.refresh_count
    }
}

pub(crate) fn shared(reset_done: Arc<Mutex<HashSet<PathBuf>>>) -> SharedClaimSelector {
    Arc::new(Mutex::new(ClaimSelector::new(reset_done)))
}

#[derive(Clone, Debug, Default)]
/// Stores state for claim error log state.
struct ClaimErrorLogState {
    failures: usize,
    last_logged: Option<Instant>,
}

/// Handles record claim error.
fn record_claim_error(
    claim_error_logs: &mut HashMap<SourceId, ClaimErrorLogState>,
    source_id: &SourceId,
    source_root: &Path,
    err: &str,
) {
    let now = Instant::now();
    let state = claim_error_logs.entry(source_id.clone()).or_default();
    state.failures = state.failures.saturating_add(1);
    let should_log = state
        .last_logged
        .is_none_or(|last_logged| now.duration_since(last_logged) >= CLAIM_ERROR_LOG_INTERVAL);
    if !should_log {
        return;
    }
    state.last_logged = Some(now);
    let source = source_root.display().to_string();
    let busy = is_busy_error(err);
    tracing::warn!(
        target: "perf::source_db",
        action = "analysis_claim_source_error",
        source_id = source_id.as_str(),
        source_root = %source_root.display(),
        failures = state.failures,
        busy,
        error = err,
        "Analysis claim source failed; continuing with other sources"
    );
    emit_db_debug_event(DbDebugEvent {
        operation: "analysis_claim_jobs.source",
        source: Some(&source),
        outcome: "error",
        elapsed: Duration::ZERO,
        error: Some(err),
    });
}

/// Handles is busy error.
fn is_busy_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}

#[cfg(test)]
/// Contains focused regression coverage for this module.
mod tests {
    use super::*;
    use crate::app::controller::library::analysis_jobs::db as analysis_db;
    use crate::sample_sources::SampleSource;
    use std::io;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    /// Stores state for shared buffer.
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedBuffer {
        /// Handles captured.
        fn captured(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl<'a> MakeWriter<'a> for SharedBuffer {
        /// Names the writer type.
        type Writer = SharedBufferWriter;

        /// Handles make writer.
        fn make_writer(&'a self) -> Self::Writer {
            SharedBufferWriter(self.0.clone())
        }
    }

    /// Stores state for shared buffer writer.
    struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for SharedBufferWriter {
        /// Handles write.
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        /// Handles flush.
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Handles capture debug logs.
    fn capture_debug_logs(run: impl FnOnce()) -> String {
        let buffer = SharedBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(buffer.clone())
            .finish();
        crate::logging::set_debug_logging_enabled_for_tests(true);
        tracing::subscriber::with_default(subscriber, run);
        crate::logging::set_debug_logging_enabled_for_tests(false);
        buffer.captured()
    }

    #[test]
    /// Handles repeated claim errors are rate limited and source scoped.
    fn repeated_claim_errors_are_rate_limited_and_source_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let source = SampleSource::new(dir.path().join("source"));
        let mut logs = HashMap::new();

        let first = capture_debug_logs(|| {
            record_claim_error(&mut logs, &source.id, &source.root, "database is locked");
        });
        assert!(
            first.contains("Analysis claim source failed; continuing with other sources"),
            "first claim error should be visible: {first}"
        );
        assert!(
            first.contains("action=\"analysis_claim_source_error\""),
            "claim error should use a stable action: {first}"
        );
        assert!(
            first.contains("busy=true"),
            "locked claim errors should be classified as busy: {first}"
        );
        assert!(
            first.contains("source_root=") && first.contains("source"),
            "claim error should include source-root context: {first}"
        );

        let second = capture_debug_logs(|| {
            record_claim_error(&mut logs, &source.id, &source.root, "database is locked");
        });
        assert!(
            !second.contains("Analysis claim source failed"),
            "immediate repeated claim errors should be rate-limited: {second}"
        );

        logs.get_mut(&source.id).unwrap().last_logged =
            Some(Instant::now() - CLAIM_ERROR_LOG_INTERVAL);
        let third = capture_debug_logs(|| {
            record_claim_error(&mut logs, &source.id, &source.root, "constraint failed");
        });
        assert!(
            third.contains("Analysis claim source failed; continuing with other sources"),
            "claim errors should log again after the interval: {third}"
        );
        assert!(
            third.contains("busy=false"),
            "non-lock claim errors should not be classified as busy: {third}"
        );
    }

    #[test]
    /// Handles empty claim sources stay quiet.
    fn empty_claim_sources_stay_quiet() {
        let dir = tempfile::tempdir().unwrap();
        let source = SampleSource::new(dir.path().join("source"));
        std::fs::create_dir_all(&source.root).unwrap();
        let conn = analysis_db::open_source_db(&source.root).unwrap();
        let mut selector = ClaimSelector::with_sources_for_tests(
            vec![super::SourceClaimDb { source, conn }],
            1,
            Arc::new(Mutex::new(HashSet::new())),
        );
        let claim_wakeup = ClaimWakeup::new();

        let captured = capture_debug_logs(|| {
            assert!(matches!(
                selector.select_next(None, &claim_wakeup),
                ClaimSelection::Idle
            ));
        });

        assert!(
            !captured.contains("analysis_claim_source_error"),
            "empty queues should not emit claim-error diagnostics: {captured}"
        );
    }
}
