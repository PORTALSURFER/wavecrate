//! Dedup tracking for analysis job claims.

use std::collections::HashSet;
use std::sync::Mutex;
use tracing::warn;

/// Shared pending/inflight dedup state protected by a single lock.
#[derive(Default)]
struct DedupState {
    pending_jobs: HashSet<i64>,
    inflight_jobs: HashSet<i64>,
}

/// Tracks inflight and pending jobs to prevent duplicate work.
pub(crate) struct DedupTracker {
    state: Mutex<DedupState>,
}

impl DedupTracker {
    /// Creates a new dedup tracker.
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(DedupState::default()),
        }
    }

    /// Marks a job inflight if it is not already inflight.
    pub(crate) fn try_mark_inflight(&self, job_id: i64) -> bool {
        let mut state = self.lock_state();
        if state.inflight_jobs.contains(&job_id) {
            return false;
        }
        state.inflight_jobs.insert(job_id);
        true
    }

    /// Clears an inflight marker for a job.
    pub(crate) fn clear_inflight(&self, job_id: i64) {
        let mut state = self.lock_state();
        state.inflight_jobs.remove(&job_id);
    }

    /// Marks a job pending if it has not already been queued.
    pub(crate) fn mark_pending(&self, job_id: i64) -> bool {
        let mut state = self.lock_state();
        if state.pending_jobs.contains(&job_id) {
            return false;
        }
        state.pending_jobs.insert(job_id);
        true
    }

    /// Clears a pending marker for a job.
    pub(crate) fn clear_pending(&self, job_id: i64) {
        let mut state = self.lock_state();
        state.pending_jobs.remove(&job_id);
    }

    /// Lock dedup state and recover from poisoned lock state while warning.
    fn lock_state(&self) -> std::sync::MutexGuard<'_, DedupState> {
        self.state.lock().unwrap_or_else(|poisoned| {
            warn!("decoded queue dedup lock poisoned; recovering.");
            poisoned.into_inner()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_pending_dedups_and_allows_reclaim() {
        let dedup = DedupTracker::new();
        assert!(dedup.mark_pending(17));
        assert!(!dedup.mark_pending(17));
        dedup.clear_pending(17);
        assert!(dedup.mark_pending(17));
    }

    #[test]
    fn inflight_prevents_duplicates_until_cleared() {
        let dedup = DedupTracker::new();
        assert!(dedup.try_mark_inflight(42));
        assert!(!dedup.try_mark_inflight(42));
        dedup.clear_inflight(42);
        assert!(dedup.try_mark_inflight(42));
    }
}
