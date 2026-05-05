use std::sync::{Condvar, Mutex};
use std::time::Duration;

struct ProgressPollerWakeupState {
    counter: u64,
}

/// Condvar-backed wakeup used to nudge the progress poller on job updates.
pub(crate) struct ProgressPollerWakeup {
    state: Mutex<ProgressPollerWakeupState>,
    ready: Condvar,
}

impl ProgressPollerWakeup {
    /// Create a new progress poller wakeup handle.
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(ProgressPollerWakeupState { counter: 0 }),
            ready: Condvar::new(),
        }
    }

    /// Notify the poller that progress state has changed.
    pub(crate) fn notify(&self) {
        let mut state = self.state.lock().expect("progress poller wakeup poisoned");
        state.counter = state.counter.wrapping_add(1);
        self.ready.notify_one();
    }

    /// Wait until notified or until the timeout elapses.
    pub(crate) fn wait_for(&self, seen: &mut u64, timeout: Duration) -> bool {
        let state = self.state.lock().expect("progress poller wakeup poisoned");
        if state.counter != *seen {
            *seen = state.counter;
            return true;
        }
        let (state, _timeout) = self
            .ready
            .wait_timeout(state, timeout)
            .expect("progress poller wakeup poisoned");
        if state.counter != *seen {
            *seen = state.counter;
            return true;
        }
        false
    }
}
