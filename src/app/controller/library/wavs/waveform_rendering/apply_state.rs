use super::*;
use crate::app::state::WaveformView;
use crate::waveform::DecodedWaveform;
use std::sync::Arc;

impl AppController {
    pub(crate) fn min_view_width(&self) -> f64 {
        if let Some(decoded) = self.sample_view.waveform.decoded.as_ref() {
            minimum_useful_view_width_for_frames(
                decoded.frame_count(),
                self.sample_view.waveform.size[0],
            )
        } else {
            MIN_VIEW_WIDTH_BASE
        }
    }

    /// Apply waveform payloads using shared immutable buffers.
    pub(crate) fn apply_waveform_image_shared(
        &mut self,
        decoded: Arc<DecodedWaveform>,
        transients: Option<Arc<[f32]>>,
    ) {
        if self
            .sample_view
            .waveform
            .decoded
            .as_ref()
            .is_some_and(|d| d.cache_token == decoded.cache_token)
        {
            // Content matches, no need to invalidate the current render or transients.
            self.sample_view.waveform.decoded = Some(decoded);
            return;
        }

        let token = decoded.cache_token;
        // Force a rerender whenever decoded samples change, even if the view metadata is
        // identical to the previous render.
        self.sample_view.waveform.render_meta = None;
        self.sample_view.waveform.decoded = Some(decoded);
        self.runtime.pending_waveform_transient_compute = None;

        // Reset view to show full waveform when loading new audio
        self.ui.waveform.view = WaveformView {
            start: 0.0,
            end: 1.0,
        };

        if let Some(transients) = transients {
            self.runtime.jobs.invalidate_waveform_transient_requests();
            self.ui.waveform.transients = transients;
            self.ui.waveform.transient_cache_token = Some(token);
        } else {
            self.ui.waveform.transients = Arc::from([]);
            self.ui.waveform.transient_cache_token = None;
            self.queue_waveform_transient_refresh(Arc::clone(
                self.sample_view
                    .waveform
                    .decoded
                    .as_ref()
                    .expect("decoded waveform should be present after assignment"),
            ));
        }
        self.refresh_waveform_image_with_reason(WaveformRefreshReason::Data);
    }

    /// Apply waveform payloads using owned values.
    ///
    /// This compatibility path adapts legacy call sites to the shared immutable
    /// payload pipeline and should be removed once all callers are Arc-first.
    pub(crate) fn apply_waveform_image(
        &mut self,
        decoded: DecodedWaveform,
        transients: Option<Vec<f32>>,
    ) {
        self.apply_waveform_image_shared(Arc::new(decoded), transients.map(Arc::from));
    }
}
