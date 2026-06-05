use super::super::*;
use crate::app::controller::playback::audio_cache::{CacheKey, CachedAudio, FileMetadata};
use crate::app::controller::playback::telemetry::{log_audio_start_stage, stage_timer};

impl AppController {
    fn memory_cached_audio_for_load(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<Option<(FileMetadata, CachedAudio)>, String> {
        let started_at = stage_timer();
        let metadata = match self.current_file_metadata(source, relative_path) {
            Ok(meta) => {
                log_audio_start_stage(
                    "memory_cache_metadata",
                    Some(&source.id),
                    Some(relative_path),
                    started_at,
                    None,
                    None,
                    None,
                    None,
                );
                meta
            }
            Err(_) => {
                log_audio_start_stage(
                    "memory_cache_metadata_failed",
                    Some(&source.id),
                    Some(relative_path),
                    started_at,
                    None,
                    None,
                    None,
                    None,
                );
                return Ok(None);
            }
        };
        let key = CacheKey::new(&source.id, relative_path);
        let cache_started_at = stage_timer();
        let hit = self.audio.cache.get(&key, metadata);
        log_audio_start_stage(
            "memory_cache_lookup",
            Some(&source.id),
            Some(relative_path),
            cache_started_at,
            hit.as_ref().map(|hit| {
                if hit.bytes.is_empty() {
                    "file"
                } else {
                    "bytes"
                }
            }),
            Some(if hit.is_some() { "hit" } else { "miss" }),
            hit.as_ref().map(|hit| hit.bytes.len()),
            hit.as_ref().map(|hit| hit.decoded.samples.len()),
        );
        Ok(hit.map(|hit| (metadata, hit)))
    }

    pub(crate) fn try_apply_cached_audio_load(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        intent: AudioLoadIntent,
    ) -> Result<bool, String> {
        if matches!(intent, AudioLoadIntent::Selection)
            && self.stretch_ratio_for_sample(relative_path).is_some()
        {
            log_audio_start_stage(
                "memory_cache_skipped_stretch",
                Some(&source.id),
                Some(relative_path),
                None,
                None,
                Some("skip"),
                None,
                None,
            );
            return Ok(false);
        }
        let Some((_metadata, hit)) = self.memory_cached_audio_for_load(source, relative_path)?
        else {
            return Ok(false);
        };
        let duration_seconds = hit.decoded.duration_seconds;
        let sample_rate = hit.decoded.sample_rate;
        self.finish_waveform_load_shared(FinishWaveformLoadShared {
            source,
            relative_path,
            decoded: hit.decoded,
            bytes: hit.bytes,
            audio_path: hit.audio_path,
            intent,
            preserve_selections: false,
            transients: Some(hit.transients),
        })?;
        self.ui.waveform.loading = None;
        self.clear_browser_selection_transition(&source.id, relative_path);
        let message = Self::loaded_status_text(relative_path, duration_seconds, sample_rate);
        self.set_status(message, StatusTone::Info);
        if matches!(intent, AudioLoadIntent::Selection) {
            self.refresh_similarity_sort_for_loaded_sample();
        }
        self.maybe_trigger_pending_playback();
        Ok(true)
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
            log_audio_start_stage(
                "memory_cache_skipped_stretch",
                Some(&source.id),
                Some(relative_path),
                None,
                None,
                Some("skip"),
                None,
                None,
            );
            return Ok(false);
        }
        let Some((_metadata, hit)) = self.memory_cached_audio_for_load(source, relative_path)?
        else {
            return Ok(false);
        };
        let duration_seconds = hit.decoded.duration_seconds;
        let sample_rate = hit.decoded.sample_rate;
        self.finish_waveform_load_shared(FinishWaveformLoadShared {
            source,
            relative_path,
            decoded: hit.decoded,
            bytes: hit.bytes,
            audio_path: hit.audio_path,
            intent,
            preserve_selections: false,
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
