use super::super::*;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::playback::audio_loader::{AudioTransientResult, AudioVisualResult};
use crate::app::controller::playback::persistent_waveform_cache::persist_waveform_cache_entry;
use std::sync::Arc;
use std::thread;

impl AppController {
    pub(crate) fn handle_audio_loaded(&mut self, pending: PendingAudio, outcome: AudioLoadOutcome) {
        let duration_seconds = outcome.decoded.duration_seconds;
        let sample_rate = outcome.decoded.sample_rate;
        self.runtime
            .jobs
            .set_staged_audio_handoff(Some(StagedAudioHandoff {
                request_id: pending.request_id,
                source_id: pending.source_id.clone(),
                root: pending.root,
                relative_path: pending.relative_path.clone(),
                intent: pending.intent,
                decoded: outcome.decoded,
                bytes: outcome.bytes,
            }));
        self.note_browser_selection_staged(&pending.source_id, &pending.relative_path);
        let message =
            Self::loaded_status_text(&pending.relative_path, duration_seconds, sample_rate);
        self.set_status(message, StatusTone::Info);
    }

    pub(crate) fn handle_audio_visual_loaded(&mut self, result: AudioVisualResult) {
        let Some(staged) = self.runtime.jobs.staged_audio_handoff() else {
            return;
        };
        if staged.request_id != result.request_id
            || staged.source_id != result.source_id
            || staged.relative_path != result.relative_path
        {
            return;
        }
        if staged.decoded.cache_token != result.cache_token {
            return;
        }
        let decoded = Arc::clone(&staged.decoded);
        let loaded_bytes = Arc::clone(&staged.bytes);
        self.ui.waveform.transients = result.transients.clone();
        self.ui.waveform.transient_cache_token = Some(result.cache_token);
        let expected_transient_visual_token = self
            .ui
            .waveform
            .transient_cache_token
            .filter(|_| self.ui.waveform.transient_markers_enabled);
        let render_matches_current_state = result
            .render_meta
            .as_ref()
            .is_some_and(|meta| meta.transient_visual_token == expected_transient_visual_token);
        if render_matches_current_state {
            self.store_prepared_waveform_image(
                result.image,
                result.projected_image,
                result.render_meta,
            );
        } else {
            self.sample_view.waveform.render_meta = None;
            self.sample_view.waveform.decoded = Some(Arc::clone(&decoded));
            self.refresh_waveform_image();
        }
        self.cache_loaded_waveform_transients(
            &result.source_id,
            &result.relative_path,
            result.metadata,
            &decoded,
            loaded_bytes,
            result.transients,
            result.stretched,
        );
        self.finalize_staged_audio_handoff(result.cache_token);
    }

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
        self.ui.waveform.transients = result.transients.clone();
        self.ui.waveform.transient_cache_token = Some(result.cache_token);
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
            result.transients,
            result.stretched,
        );
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
        self.runtime.jobs.set_staged_audio_handoff(None);
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
        self.clear_browser_selection_transition(&pending.source_id, &pending.relative_path);
    }

    fn apply_loaded_audio_handoff(
        &mut self,
        source: &SampleSource,
        handoff: &StagedAudioHandoff,
    ) -> Result<(), String> {
        let relative_path = handoff.relative_path.as_path();
        let duration_seconds = handoff.decoded.duration_seconds;
        let sample_rate = handoff.decoded.sample_rate;
        self.sample_view.waveform.decoded = Some(Arc::clone(&handoff.decoded));
        self.sample_view.wav.loaded_wav = Some(handoff.relative_path.clone());
        self.set_ui_loaded_wav(Some(handoff.relative_path.clone()));
        self.sync_loaded_audio(
            source,
            relative_path,
            duration_seconds,
            sample_rate,
            Arc::clone(&handoff.bytes),
        )?;
        self.ui.waveform.notice = None;
        if matches!(handoff.intent, AudioLoadIntent::Selection) {
            self.apply_loaded_sample_bpm(relative_path);
            self.apply_loaded_sample_loop_marker(source, relative_path);
        }
        Ok(())
    }

    /// Publish one staged audio load once waveform visuals for the same decode are ready.
    pub(crate) fn finalize_staged_audio_handoff(&mut self, cache_token: u64) {
        if self.runtime.pending_waveform_render.is_some() {
            return;
        }
        let Some(staged) = self.runtime.jobs.staged_audio_handoff() else {
            return;
        };
        if staged.decoded.cache_token != cache_token {
            return;
        }
        let source = SampleSource {
            id: staged.source_id.clone(),
            root: staged.root.clone(),
        };
        if let Err(err) = self.apply_loaded_audio_handoff(&source, &staged) {
            self.runtime.jobs.set_staged_audio_handoff(None);
            self.runtime.jobs.set_pending_playback(None);
            self.clear_browser_selection_transition(&staged.source_id, &staged.relative_path);
            self.set_status(err, StatusTone::Error);
            return;
        }
        self.runtime.jobs.set_staged_audio_handoff(None);
        self.ui.waveform.loading = None;
        self.clear_browser_selection_transition(&staged.source_id, &staged.relative_path);
        if matches!(staged.intent, AudioLoadIntent::Selection) {
            self.refresh_similarity_sort_for_loaded_sample();
        }
        self.maybe_trigger_pending_playback();
    }

    fn cache_loaded_waveform_transients(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        metadata: crate::app::controller::playback::audio_cache::FileMetadata,
        decoded: &Arc<DecodedWaveform>,
        loaded_bytes: Arc<[u8]>,
        transients: Arc<[f32]>,
        stretched: bool,
    ) {
        if stretched {
            return;
        }
        let key = CacheKey::new(source_id, relative_path);
        self.audio.cache.insert(
            key,
            metadata,
            Arc::clone(decoded),
            loaded_bytes,
            transients.clone(),
        );
        let source_id = source_id.clone();
        let relative_path = relative_path.to_path_buf();
        let decoded = Arc::clone(decoded);
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
