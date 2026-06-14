use super::super::super::*;
use crate::app::controller::playback::audio_loader::AudioTransientResult;
use std::sync::Arc;

impl AppController {
    pub(crate) fn handle_audio_transients_loaded(&mut self, result: AudioTransientResult) {
        let Some(loaded_audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return;
        };
        if loaded_audio.source_id != result.source_id
            || loaded_audio.relative_path != result.relative_path
        {
            return;
        }
        let loaded_bytes = Arc::clone(&loaded_audio.bytes);
        let Some(decoded) = self.sample_view.waveform.decoded.as_ref() else {
            return;
        };
        if decoded.cache_token != result.cache_token {
            return;
        }
        let decoded = Arc::clone(decoded);
        self.publish_loaded_transients(result.cache_token, result.transients.clone());
        if self.ui.waveform.transient_markers_enabled {
            self.sample_view.waveform.render_meta = None;
            self.refresh_waveform_image();
        }
        self.cache_loaded_waveform_transients(
            &result.source_id,
            &result.relative_path,
            result.metadata,
            &decoded,
            loaded_bytes,
            None,
            result.transients,
            result.stretched,
        );
    }

    pub(super) fn publish_loaded_transients(&mut self, cache_token: u64, transients: Arc<[f32]>) {
        self.ui.waveform.transients = transients;
        self.ui.waveform.transient_cache_token = Some(cache_token);
    }
}
