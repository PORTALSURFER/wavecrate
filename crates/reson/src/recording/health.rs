//! Lock-free recording health counters shared with the capture callback.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Observable health counters for one recording session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordingHealth {
    /// Samples dropped because the bounded WAV-writer ring was full.
    pub writer_dropped_samples: u64,
    /// Capture callbacks that encountered a full WAV-writer ring.
    pub writer_overrun_events: u64,
    /// Samples dropped because the bounded monitor ring was full.
    pub monitor_dropped_samples: u64,
    /// Capture callbacks that encountered a full monitor ring.
    pub monitor_overrun_events: u64,
    /// Monitor attach/detach commands rejected by the bounded control queue.
    pub monitor_control_drops: u64,
    /// Whether the WAV writer exited with an error.
    pub writer_failed: bool,
    /// Whether the monitor worker exited unexpectedly.
    pub monitor_failed: bool,
    /// Whether the monitor control worker disconnected.
    pub monitor_disconnected: bool,
}

#[derive(Default)]
pub(super) struct RecordingHealthState {
    pub(super) writer_dropped_samples: AtomicU64,
    pub(super) writer_overrun_events: AtomicU64,
    pub(super) monitor_dropped_samples: AtomicU64,
    pub(super) monitor_overrun_events: AtomicU64,
    pub(super) monitor_control_drops: AtomicU64,
    pub(super) writer_failed: AtomicBool,
    pub(super) monitor_failed: AtomicBool,
    pub(super) monitor_disconnected: AtomicBool,
}

impl RecordingHealthState {
    pub(super) fn snapshot(&self) -> RecordingHealth {
        RecordingHealth {
            writer_dropped_samples: self.writer_dropped_samples.load(Ordering::Relaxed),
            writer_overrun_events: self.writer_overrun_events.load(Ordering::Relaxed),
            monitor_dropped_samples: self.monitor_dropped_samples.load(Ordering::Relaxed),
            monitor_overrun_events: self.monitor_overrun_events.load(Ordering::Relaxed),
            monitor_control_drops: self.monitor_control_drops.load(Ordering::Relaxed),
            writer_failed: self.writer_failed.load(Ordering::Acquire),
            monitor_failed: self.monitor_failed.load(Ordering::Acquire),
            monitor_disconnected: self.monitor_disconnected.load(Ordering::Acquire),
        }
    }
}
