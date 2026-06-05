use super::super::*;
use crate::app::controller::playback::audio_cache::{CacheKey, CachedAudio, FileMetadata};
use crate::app::controller::playback::audio_loader::PreparedAudioLoad;

impl AppController {
    fn memory_cached_audio_for_load(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<Option<(FileMetadata, CachedAudio)>, String> {
        let metadata = match self.current_file_metadata(source, relative_path) {
            Ok(meta) => meta,
            Err(_) => return Ok(None),
        };
        let key = CacheKey::new(&source.id, relative_path);
        Ok(self
            .audio
            .cache
            .get(&key, metadata)
            .map(|hit| (metadata, hit)))
    }

    pub(crate) fn try_queue_cached_audio_load(
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
        let Some((metadata, hit)) = self.memory_cached_audio_for_load(source, relative_path)?
        else {
            return Ok(false);
        };
        let request_id = self.runtime.jobs.next_audio_request_id();
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
            stretch_ratio: None,
            render_spec: self.initial_waveform_render_spec(),
            prepared: Some(PreparedAudioLoad {
                metadata,
                decoded: hit.decoded,
                bytes: hit.bytes,
                transients: hit.transients,
                stretched: false,
            }),
        };
        if self.runtime.jobs.send_audio_job(job).is_err() {
            return Err("Failed to queue cached audio load".to_string());
        }
        self.runtime.jobs.set_pending_audio(Some(pending));
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
