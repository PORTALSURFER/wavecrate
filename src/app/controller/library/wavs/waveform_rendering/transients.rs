use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::state::runtime::PendingWaveformTransientCompute;
use crate::waveform::DecodedWaveform;
use std::sync::Arc;
use std::time::Instant;

impl AppController {
    pub(super) fn queue_waveform_transient_refresh(&mut self, decoded: Arc<DecodedWaveform>) {
        if self.ui.waveform.transient_cache_token == Some(decoded.cache_token) {
            return;
        }
        if self
            .runtime
            .pending_waveform_transient_compute
            .as_ref()
            .is_some_and(|pending| pending.cache_token == decoded.cache_token)
        {
            return;
        }
        let request_id = self.runtime.jobs.next_waveform_transient_request_id();
        self.runtime.pending_waveform_transient_compute = Some(PendingWaveformTransientCompute {
            request_id,
            cache_token: decoded.cache_token,
            queued_at: Instant::now(),
        });
        self.runtime
            .jobs
            .publish_latest_waveform_transient_request_id(request_id);
        let latest_request_id = self
            .runtime
            .jobs
            .latest_waveform_transient_request_tracker();
        self.runtime
            .jobs
            .spawn_optional_one_shot_job(true, move || {
                worker_jobs::run_waveform_transient_job(request_id, decoded, latest_request_id)
                    .map(JobMessage::WaveformTransientsComputed)
            });
    }

    pub(crate) fn refresh_waveform_transients(&mut self) {
        let Some(decoded) = self.sample_view.waveform.decoded.as_ref().cloned() else {
            self.ui.waveform.transients = Arc::from([]);
            self.ui.waveform.transient_cache_token = None;
            return;
        };
        self.ui.waveform.transients = Arc::from([]);
        self.ui.waveform.transient_cache_token = None;
        self.queue_waveform_transient_refresh(decoded);
    }
}
