use super::super::*;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::playback::audio_loader::{AudioTransientResult, AudioVisualResult};
use crate::app::controller::playback::persistent_waveform_cache::persist_waveform_cache_entry;
use std::sync::Arc;
use std::thread;

impl AppController {
    pub(crate) fn handle_audio_loaded(&mut self, pending: PendingAudio, outcome: AudioLoadOutcome) {
        let source = SampleSource {
            id: pending.source_id.clone(),
            root: pending.root.clone(),
        };
        let duration_seconds = outcome.decoded.duration_seconds;
        let sample_rate = outcome.decoded.sample_rate;
        if let Err(err) = self.apply_loaded_audio_primary(
            &source,
            &pending.relative_path,
            outcome.decoded,
            outcome.bytes,
            pending.intent,
        ) {
            self.runtime.jobs.set_pending_playback(None);
            self.set_status(err, StatusTone::Error);
            return;
        }
        let message =
            Self::loaded_status_text(&pending.relative_path, duration_seconds, sample_rate);
        self.set_status(message, StatusTone::Info);
        if matches!(pending.intent, AudioLoadIntent::Selection) {
            self.refresh_similarity_sort_for_loaded_sample();
        }
        self.maybe_trigger_pending_playback();
    }

    pub(crate) fn handle_audio_visual_loaded(&mut self, result: AudioVisualResult) {
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
        self.ui.waveform.transients = result.transients.clone();
        self.ui.waveform.transient_cache_token = Some(result.cache_token);
        self.store_prepared_waveform_image(
            result.image,
            result.projected_image,
            result.render_meta,
        );
        self.ui.waveform.loading = None;
        if !result.stretched {
            let key = CacheKey::new(&result.source_id, &result.relative_path);
            self.audio.cache.insert(
                key,
                result.metadata,
                Arc::clone(&decoded),
                loaded_bytes,
                result.transients.clone(),
            );
            let source_id = result.source_id.clone();
            let relative_path = result.relative_path.clone();
            let metadata = result.metadata;
            let decoded = Arc::clone(&decoded);
            let transients = result.transients.clone();
            thread::spawn(move || {
                persist_waveform_cache_entry(
                    &source_id,
                    &relative_path,
                    metadata,
                    &decoded,
                    &transients,
                );
            });
        }
    }

    pub(crate) fn handle_audio_transients_loaded(&mut self, result: AudioTransientResult) {
        self.handle_audio_visual_loaded(AudioVisualResult {
            request_id: result.request_id,
            source_id: result.source_id,
            relative_path: result.relative_path,
            metadata: result.metadata,
            cache_token: result.cache_token,
            transients: result.transients,
            image: None,
            projected_image: None,
            render_meta: None,
            stretched: result.stretched,
        });
    }

    pub(crate) fn handle_audio_load_error(&mut self, pending: PendingAudio, error: AudioLoadError) {
        let source = SampleSource {
            id: pending.source_id.clone(),
            root: pending.root.clone(),
        };
        if self
            .runtime
            .jobs
            .pending_playback()
            .as_ref()
            .is_some_and(|pending_play| {
                pending_play.source_id == pending.source_id
                    && pending_play.relative_path == pending.relative_path
            })
        {
            self.runtime.jobs.set_pending_playback(None);
        }
        match error {
            AudioLoadError::Missing(msg) => {
                let _ = self.prune_missing_sample(&source, &pending.relative_path);
                self.show_missing_waveform_notice(&pending.relative_path);
                self.set_status(msg, StatusTone::Warning);
            }
            AudioLoadError::Failed(msg) => {
                self.set_status(msg, StatusTone::Error);
            }
        }
        self.ui.waveform.loading = None;
    }

    fn apply_loaded_audio_primary(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        decoded: Arc<DecodedWaveform>,
        bytes: Arc<[u8]>,
        intent: AudioLoadIntent,
    ) -> Result<(), String> {
        let duration_seconds = decoded.duration_seconds;
        let sample_rate = decoded.sample_rate;
        self.sample_view.waveform.decoded = Some(decoded);
        self.sample_view.wav.loaded_wav = Some(relative_path.to_path_buf());
        self.set_ui_loaded_wav(Some(relative_path.to_path_buf()));
        self.sync_loaded_audio(source, relative_path, duration_seconds, sample_rate, bytes)?;
        self.ui.waveform.notice = None;
        if matches!(intent, AudioLoadIntent::Selection) {
            self.apply_loaded_sample_bpm(relative_path);
            self.apply_loaded_sample_loop_marker(source, relative_path);
            self.refresh_similarity_sort_for_loaded_sample();
        }
        Ok(())
    }
}
