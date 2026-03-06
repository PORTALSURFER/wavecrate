use super::*;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::playback::audio_loader::AudioTransientResult;
use std::path::Path;
use std::sync::Arc;

impl AppController {
    pub(crate) fn handle_audio_loaded(&mut self, pending: PendingAudio, outcome: AudioLoadOutcome) {
        let source = SampleSource {
            id: pending.source_id.clone(),
            root: pending.root.clone(),
        };
        let (decoded, bytes, stretched) = if outcome.stretched {
            (outcome.decoded, outcome.bytes, true)
        } else {
            match self.prepare_loaded_audio(
                &pending.relative_path,
                Some(outcome.decoded),
                outcome.bytes,
                pending.intent,
            ) {
                Ok(result) => result,
                Err(err) => {
                    self.runtime.jobs.set_pending_playback(None);
                    self.set_status(err, StatusTone::Error);
                    return;
                }
            }
        };
        let duration_seconds = decoded.duration_seconds;
        let sample_rate = decoded.sample_rate;
        let cache_key = CacheKey::new(&source.id, &pending.relative_path);
        if !stretched {
            self.audio.cache.insert(
                cache_key,
                outcome.metadata,
                decoded.clone(),
                bytes.clone(),
                std::sync::Arc::from([]),
            );
        }
        let preserve_selections =
            self.sample_view.wav.loaded_wav.as_deref() == Some(&pending.relative_path);
        if let Err(err) = self.finish_waveform_load_shared(FinishWaveformLoadShared {
            source: &source,
            relative_path: &pending.relative_path,
            decoded,
            bytes,
            intent: pending.intent,
            preserve_selections,
            transients: Some(std::sync::Arc::from([])),
        }) {
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

    /// Apply deferred transient markers if they still match the active loaded waveform.
    pub(crate) fn handle_audio_transients_loaded(&mut self, result: AudioTransientResult) {
        let Some(loaded_audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return;
        };
        if loaded_audio.source_id != result.source_id
            || loaded_audio.relative_path != result.relative_path
        {
            return;
        }
        let Some(decoded) = self.sample_view.waveform.decoded.as_ref() else {
            return;
        };
        if decoded.cache_token != result.cache_token {
            return;
        }
        self.ui.waveform.transients = result.transients.clone();
        self.ui.waveform.transient_cache_token = Some(result.cache_token);
        if !result.stretched {
            let key = CacheKey::new(&result.source_id, &result.relative_path);
            self.audio
                .cache
                .update_transients(&key, result.metadata, result.transients.clone());
            if let Some(decoded) = self.sample_view.waveform.decoded.as_ref() {
                self.persist_waveform_cache(
                    &result.source_id,
                    &result.relative_path,
                    result.metadata,
                    decoded,
                    &result.transients,
                );
            }
        }
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
                self.mark_sample_missing(&source, &pending.relative_path);
                self.show_missing_waveform_notice(&pending.relative_path);
                self.set_status(msg, StatusTone::Warning);
            }
            AudioLoadError::Failed(msg) => {
                self.set_status(msg, StatusTone::Error);
            }
        }
    }

    pub(crate) fn maybe_trigger_pending_playback(&mut self) {
        let Some(pending) = self.runtime.jobs.pending_playback() else {
            return;
        };
        let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return;
        };
        if audio.source_id != pending.source_id || audio.relative_path != pending.relative_path {
            return;
        }
        self.runtime.jobs.set_pending_playback(None);
        if let Err(err) = self.play_audio(pending.looped, pending.start_override) {
            self.set_status(err, StatusTone::Error);
        }
    }

    pub(crate) fn queue_audio_load_for(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        intent: AudioLoadIntent,
        pending_playback: Option<PendingPlayback>,
    ) -> Result<(), String> {
        let request_id = self.runtime.jobs.next_audio_request_id();
        let stretch_ratio = self.stretch_ratio_for_sample(relative_path);
        let pending = PendingAudio {
            request_id,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            intent,
        };
        let job = AudioLoadJob {
            request_id,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            stretch_ratio,
        };
        self.runtime.jobs.set_pending_audio(None);
        self.runtime.jobs.set_pending_playback(pending_playback);
        self.ui.waveform.loading = Some(relative_path.to_path_buf());
        self.ui.waveform.waveform_image_signature = None;
        self.projected_waveform_image_signature = None;
        self.projected_waveform_image = None;
        self.ui.waveform.notice = None;
        self.sample_view.waveform.render_meta = None;
        self.sample_view.waveform.decoded = None;
        self.ui.waveform.image = None;
        self.sample_view.wav.loaded_audio = None;
        self.sample_view.wav.loaded_wav = None;
        self.set_ui_loaded_wav(None);
        self.stop_playback_if_active();
        self.clear_waveform_selection();
        self.set_status(
            format!("Loading {}", relative_path.display()),
            StatusTone::Busy,
        );
        if self.try_use_cached_audio(source, relative_path, intent)? {
            self.maybe_trigger_pending_playback();
            return Ok(());
        }
        if self.runtime.jobs.send_audio_job(job).is_err() {
            self.runtime.jobs.set_pending_audio(None);
            self.runtime.jobs.set_pending_playback(None);
            self.ui.waveform.loading = None;
            return Err("Failed to queue audio load".to_string());
        }
        self.runtime.jobs.set_pending_audio(Some(pending));
        Ok(())
    }

    pub(crate) fn try_use_cached_audio(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        intent: AudioLoadIntent,
    ) -> Result<bool, String> {
        if matches!(intent, AudioLoadIntent::Selection)
            && self.stretch_ratio_for_sample(relative_path).is_some()
        {
            return Ok(false);
        }
        let metadata = match self.current_file_metadata(source, relative_path) {
            Ok(meta) => meta,
            Err(_) => return Ok(false),
        };
        let key = CacheKey::new(&source.id, relative_path);
        let Some(hit) = self.audio.cache.get(&key, metadata) else {
            let Some(hit) =
                self.load_persistent_waveform_cache(&source.id, relative_path, metadata)
            else {
                return Ok(false);
            };
            let bytes: Arc<[u8]> = match self.read_waveform_bytes(source, relative_path) {
                Ok(bytes) => Arc::from(bytes),
                Err(err) => {
                    tracing::warn!(
                        "Failed to hydrate waveform cache bytes for {}: {err}",
                        relative_path.display()
                    );
                    return Ok(false);
                }
            };
            self.audio.cache.insert(
                key,
                metadata,
                hit.decoded.clone(),
                bytes.clone(),
                hit.transients.clone(),
            );
            let duration_seconds = hit.decoded.duration_seconds;
            let sample_rate = hit.decoded.sample_rate;
            let preserve_selections =
                self.sample_view.wav.loaded_wav.as_deref() == Some(relative_path);
            self.finish_waveform_load_shared(FinishWaveformLoadShared {
                source,
                relative_path,
                decoded: hit.decoded,
                bytes,
                intent,
                preserve_selections,
                transients: Some(hit.transients),
            })?;
            let message = Self::loaded_status_text(relative_path, duration_seconds, sample_rate);
            self.set_status(message, StatusTone::Info);
            if matches!(intent, AudioLoadIntent::Selection) {
                self.refresh_similarity_sort_for_loaded_sample();
            }
            return Ok(true);
        };
        let duration_seconds = hit.decoded.duration_seconds;
        let sample_rate = hit.decoded.sample_rate;
        let preserve_selections = self.sample_view.wav.loaded_wav.as_deref() == Some(relative_path);
        self.finish_waveform_load_shared(FinishWaveformLoadShared {
            source,
            relative_path,
            decoded: hit.decoded,
            bytes: hit.bytes,
            intent,
            preserve_selections,
            transients: Some(hit.transients),
        })?;
        let message = Self::loaded_status_text(relative_path, duration_seconds, sample_rate);
        self.set_status(message, StatusTone::Info);
        if matches!(intent, AudioLoadIntent::Selection) {
            self.refresh_similarity_sort_for_loaded_sample();
        }
        Ok(true)
    }
}
