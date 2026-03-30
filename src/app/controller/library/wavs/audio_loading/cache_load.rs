use super::super::*;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::playback::audio_loader::PreparedAudioLoad;

impl AppController {
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
        let metadata = match self.current_file_metadata(source, relative_path) {
            Ok(meta) => meta,
            Err(_) => return Ok(false),
        };
        let key = CacheKey::new(&source.id, relative_path);
        let Some(hit) = self.audio.cache.get(&key, metadata) else {
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
            self.finish_waveform_load_shared(FinishWaveformLoadShared {
                source,
                relative_path,
                decoded: hit.decoded,
                bytes,
                intent,
                preserve_selections: false,
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
