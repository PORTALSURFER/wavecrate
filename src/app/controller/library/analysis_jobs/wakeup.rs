//! Shared wakeup helpers for analysis job claiming.

use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::{Duration, Instant};

struct ClaimWakeupState {
    counter: u64,
    probe_inflight: bool,
    next_probe_at: Instant,
}

/// Condvar-backed wakeup used to notify analysis job claimers.
pub(crate) struct ClaimWakeup {
    state: Mutex<ClaimWakeupState>,
    ready: Condvar,
}

impl ClaimWakeup {
    /// Create a new wakeup handle for analysis job claimers.
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(ClaimWakeupState {
                counter: 0,
                probe_inflight: false,
                next_probe_at: Instant::now(),
            }),
            ready: Condvar::new(),
        }
    }

    /// Notify claimers that new work or capacity is available.
    pub(crate) fn notify(&self) {
        let mut state = self.state.lock().expect("claim wakeup poisoned");
        state.counter = state.counter.wrapping_add(1);
        state.probe_inflight = false;
        state.next_probe_at = Instant::now();
        self.ready.notify_all();
    }

    /// Acquire the shared idle probe permit or return the remaining backoff.
    pub(crate) fn acquire_probe_or_wait(&self, seen: &mut u64) -> Option<Duration> {
        let mut state = self.state.lock().expect("claim wakeup poisoned");
        if state.counter != *seen {
            *seen = state.counter;
        }
        let now = Instant::now();
        if !state.probe_inflight && now >= state.next_probe_at {
            state.probe_inflight = true;
            return None;
        }
        Some(state.next_probe_at.saturating_duration_since(now))
    }

    /// Release the shared idle probe permit and set the next probe backoff.
    pub(crate) fn finish_probe(&self, backoff: Duration) {
        let mut state = self.state.lock().expect("claim wakeup poisoned");
        state.probe_inflight = false;
        state.next_probe_at = Instant::now() + backoff;
        self.ready.notify_all();
    }

    /// Wait until notified or the timeout elapses.
    pub(crate) fn wait_for(&self, seen: &mut u64, timeout: Duration) -> bool {
        let state = self.state.lock().expect("claim wakeup poisoned");
        if state.counter != *seen {
            *seen = state.counter;
            return true;
        }
        let (state, _timeout) = self
            .ready
            .wait_timeout(state, timeout)
            .expect("claim wakeup poisoned");
        if state.counter != *seen {
            *seen = state.counter;
            return true;
        }
        false
    }

    /// Return the current notification counter.
    #[cfg(test)]
    pub(crate) fn snapshot(&self) -> u64 {
        self.state.lock().expect("claim wakeup poisoned").counter
    }

    #[cfg(test)]
    pub(crate) fn probe_inflight(&self) -> bool {
        self.state
            .lock()
            .expect("claim wakeup poisoned")
            .probe_inflight
    }
}

static CLAIM_WAKEUP: LazyLock<Arc<ClaimWakeup>> = LazyLock::new(|| Arc::new(ClaimWakeup::new()));

/// Return the shared wakeup handle used by the analysis claim workers.
pub(crate) fn claim_wakeup_handle() -> Arc<ClaimWakeup> {
    Arc::clone(&CLAIM_WAKEUP)
}

/// Notify analysis claim workers that new jobs are available.
pub(crate) fn notify_claim_wakeup() {
    CLAIM_WAKEUP.notify();
}
