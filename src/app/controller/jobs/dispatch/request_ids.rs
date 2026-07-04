use super::*;

fn next_request_id(counter: &mut u64) -> u64 {
    let request_id = *counter;
    *counter = counter.wrapping_add(1).max(1);
    request_id
}

impl ControllerJobs {
    /// Generate a request id for source hydration jobs.
    pub(in super::super::super) fn next_source_hydration_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_source_hydration_request_id)
    }

    /// Generate a request id for source add preparation jobs.
    pub(in super::super::super) fn next_source_add_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_source_add_request_id)
    }

    /// Generate a request id for pane-scoped folder projection jobs.
    pub(in super::super::super) fn next_folder_projection_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_folder_projection_request_id)
    }

    /// Generate a request id for async browser feature-cache refresh jobs.
    pub(in super::super::super) fn next_feature_cache_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_feature_cache_request_id)
    }

    /// Generate a request id for optimistic metadata mutation jobs.
    pub(in super::super::super) fn next_metadata_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_metadata_request_id)
    }

    /// Generate a request id for background waveform image renders.
    pub(in super::super::super) fn next_waveform_render_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_waveform_render_request_id)
    }

    /// Publish the latest queued waveform-render request id for stale-work dropping.
    pub(in super::super::super) fn publish_latest_waveform_render_request_id(
        &self,
        request_id: u64,
    ) {
        self.latest_waveform_render_request_id
            .store(request_id, Ordering::Relaxed);
    }

    /// Clone the latest waveform-render request tracker for worker-side stale checks.
    pub(in super::super::super) fn latest_waveform_render_request_tracker(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.latest_waveform_render_request_id)
    }

    /// Invalidate any in-flight waveform-render request so stale workers self-drop.
    pub(in super::super::super) fn invalidate_waveform_render_requests(&self) {
        self.latest_waveform_render_request_id
            .store(0, Ordering::Relaxed);
    }

    /// Generate a request id for deferred waveform transient computation jobs.
    pub(in super::super::super) fn next_waveform_transient_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_waveform_transient_request_id)
    }

    /// Publish the latest queued waveform-transient request id for stale-work dropping.
    pub(in super::super::super) fn publish_latest_waveform_transient_request_id(
        &self,
        request_id: u64,
    ) {
        self.latest_waveform_transient_request_id
            .store(request_id, Ordering::Relaxed);
    }

    /// Clone the latest waveform-transient request tracker for worker-side stale checks.
    pub(in super::super::super) fn latest_waveform_transient_request_tracker(
        &self,
    ) -> Arc<AtomicU64> {
        Arc::clone(&self.latest_waveform_transient_request_id)
    }

    /// Invalidate any in-flight waveform-transient request so stale workers self-drop.
    pub(in super::super::super) fn invalidate_waveform_transient_requests(&self) {
        self.latest_waveform_transient_request_id
            .store(0, Ordering::Relaxed);
    }

    /// Generate a request id for deferred configuration persistence jobs.
    pub(in super::super::super) fn next_config_persist_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_config_persist_request_id)
    }

    /// Generate a request id for source hydration jobs.
    pub(in super::super::super) fn next_audio_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_audio_request_id)
    }

    /// Generate a request id for recording waveform refresh jobs.
    pub(in super::super::super) fn next_recording_waveform_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_recording_waveform_request_id)
    }

    /// Generate a request id for controller-owned similarity query jobs.
    pub(in super::super::super) fn next_similarity_request_id(&mut self) -> u64 {
        next_request_id(&mut self.request_counters.next_similarity_request_id)
    }
}
