//! Runtime state for waveform refresh, rendering, transient analysis, and seek coalescing.

use crate::app::controller::jobs;
use std::time::Instant;

/// Classified causes for queued waveform image refresh work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformRefreshReason {
    /// Waveform sample content changed and requires a rerender.
    Data,
    /// Waveform view window/cursor/selection changed.
    View,
    /// Waveform render target dimensions changed.
    Size,
}

/// Waveform-related runtime state owned by controller frame preparation.
#[derive(Clone, Debug)]
pub(crate) struct WaveformRuntimeState {
    /// True when a waveform image rebuild is queued for the next frame prep.
    pub(crate) refresh_pending: bool,
    /// Last known cause for a queued waveform refresh request.
    pub(crate) refresh_pending_reason: Option<WaveformRefreshReason>,
    /// Nesting depth for waveform refresh batching.
    refresh_batch_depth: u16,
    /// Latest queued waveform render request, when any.
    pub(crate) pending_render: Option<PendingWaveformRender>,
    /// Latest queued waveform transient compute request, when any.
    pub(crate) pending_transient_compute: Option<PendingWaveformTransientCompute>,
    /// Latest queued waveform seek target from high-frequency interaction updates.
    pub(crate) pending_seek_nanos: Option<u32>,
    /// Earliest frame time when a deferred waveform seek commit may run.
    pub(crate) pending_seek_not_before: Option<Instant>,
    /// Monotonic producer-side id for newly rendered waveform image payloads.
    pub(crate) next_image_signature: u64,
}

impl Default for WaveformRuntimeState {
    fn default() -> Self {
        Self {
            refresh_pending: false,
            refresh_pending_reason: None,
            refresh_batch_depth: 0,
            pending_render: None,
            pending_transient_compute: None,
            pending_seek_nanos: None,
            pending_seek_not_before: None,
            next_image_signature: 1,
        }
    }
}

impl WaveformRuntimeState {
    /// Begin a waveform-refresh batch where refresh requests are coalesced.
    pub(crate) fn begin_refresh_batch(&mut self) {
        self.refresh_batch_depth = self.refresh_batch_depth.saturating_add(1);
    }

    /// End the current waveform-refresh batch, saturating at zero depth.
    pub(crate) fn end_refresh_batch(&mut self) {
        self.refresh_batch_depth = self.refresh_batch_depth.saturating_sub(1);
    }

    /// Return true when waveform refresh requests should be deferred.
    pub(crate) fn refresh_batch_active(&self) -> bool {
        self.refresh_batch_depth > 0
    }
}

/// Latest-only waveform render request owned by the controller.
#[derive(Clone, Debug)]
pub(crate) struct PendingWaveformRender {
    /// Request id used to discard stale completions.
    pub(crate) request_id: u64,
    /// Stable render key used for staleness checks.
    pub(crate) key: jobs::WaveformRenderKey,
    /// Time when the render request was queued.
    pub(crate) queued_at: Instant,
}

/// Latest-only waveform transient compute request owned by the controller.
#[derive(Clone, Debug)]
pub(crate) struct PendingWaveformTransientCompute {
    /// Request id used to discard stale completions.
    pub(crate) request_id: u64,
    /// Decode cache token used for staleness checks.
    pub(crate) cache_token: u64,
    /// Time when the transient request was queued.
    pub(crate) queued_at: Instant,
}

#[cfg(test)]
mod tests {
    use super::WaveformRuntimeState;

    #[test]
    /// Waveform runtime should start idle with image signatures reserved from one.
    fn default_waveform_runtime_is_idle() {
        let state = WaveformRuntimeState::default();
        assert!(!state.refresh_pending);
        assert!(state.refresh_pending_reason.is_none());
        assert!(!state.refresh_batch_active());
        assert!(state.pending_render.is_none());
        assert!(state.pending_transient_compute.is_none());
        assert!(state.pending_seek_nanos.is_none());
        assert_eq!(state.next_image_signature, 1);
    }

    #[test]
    fn waveform_refresh_batches_saturate_at_zero() {
        let mut state = WaveformRuntimeState::default();
        state.begin_refresh_batch();
        assert!(state.refresh_batch_active());
        state.end_refresh_batch();
        state.end_refresh_batch();
        assert!(!state.refresh_batch_active());
    }
}
