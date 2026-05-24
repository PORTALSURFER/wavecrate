use super::*;
use crate::waveform::DecodedWaveform;
use std::sync::Arc;

impl AppController {
    pub(super) fn apply_waveform_slide_preview(
        &mut self,
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
    ) {
        let channels = channels.max(1);
        let total_frames = samples.len() / channels as usize;
        if total_frames == 0 {
            return;
        }
        let duration_seconds = total_frames as f32 / sample_rate.max(1) as f32;
        let cache_token = crate::waveform::next_cache_token();
        self.sample_view.waveform.decoded = Some(Arc::new(DecodedWaveform {
            cache_token,
            samples: Arc::from(samples),
            analysis_samples: Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds,
            sample_rate: sample_rate.max(1),
            channels,
        }));
        self.sample_view.waveform.render_meta = None;
        self.ui.waveform.transient_cache_token = None;
        self.refresh_waveform_image();
    }
}
