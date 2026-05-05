//! Claim selection helpers for analysis jobs.

use super::claim::{SourceClaimDb, claim_batch_size, refresh_sources};
use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::wakeup::ClaimWakeup;
use crate::app::controller::library::source_write_priority;
use crate::sample_sources::SourceId;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
            let jobs = match db::claim_next_jobs(
                &mut source.conn,
                &source.source.root,
                self.claim_batch,
            ) {
                Ok(jobs) => jobs,
                Err(_) => continue,
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
