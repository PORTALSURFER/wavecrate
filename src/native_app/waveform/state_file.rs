use std::{path::PathBuf, sync::Arc};

use super::{WaveformFile, WaveformState};

impl WaveformState {
    pub(in crate::native_app) fn file(&self) -> Arc<WaveformFile> {
        Arc::clone(&self.file)
    }

    pub(in crate::native_app) fn sample_rate(&self) -> u32 {
        self.file.sample_rate
    }

    pub(in crate::native_app) fn channels(&self) -> usize {
        self.file.channels
    }

    pub(in crate::native_app) fn frames(&self) -> usize {
        self.file.frames
    }

    pub(in crate::native_app) fn duration_seconds(&self) -> f32 {
        self.file.frames as f32 / self.file.sample_rate.max(1) as f32
    }

    pub(in crate::native_app) fn file_name(&self) -> String {
        if self.file.path.as_os_str().is_empty() {
            return String::from("No sample loaded");
        }
        self.file
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.file.path.display().to_string())
    }

    pub(in crate::native_app) fn path(&self) -> PathBuf {
        self.file.path.clone()
    }

    pub(in crate::native_app) fn rewrite_path_prefix(
        &mut self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> bool {
        if !Arc::make_mut(&mut self.file).rewrite_path_prefix(old_path, new_path) {
            return false;
        }
        self.detail_summary = None;
        self.pending_detail_key = None;
        self.failed_detail_key = None;
        true
    }

    pub(in crate::native_app) fn has_loaded_sample(&self) -> bool {
        self.file.has_loaded_sample_metadata()
    }

    #[cfg(test)]
    pub(in crate::native_app) fn signal_summary_peak_for_tests(&self) -> f32 {
        self.file
            .gpu_signal_summary
            .levels
            .iter()
            .flat_map(|level| level.buckets.iter())
            .fold(0.0_f32, |peak, bucket| {
                peak.max(bucket.min.abs()).max(bucket.max.abs())
            })
    }

    pub(in crate::native_app) fn audio_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.file.audio_bytes)
    }

    pub(in crate::native_app) fn playback_source_file(&self) -> Option<PathBuf> {
        self.file_backed_playback_available()
            .then(|| self.file.path.clone())
    }

    pub(in crate::native_app) fn playback_samples(&self) -> Option<Arc<[f32]>> {
        self.file.playback_samples.as_ref().map(Arc::clone)
    }

    pub(in crate::native_app) fn playback_cache_file(
        &self,
    ) -> Option<super::audio_file::PersistedPlaybackCacheFile> {
        self.file.playback_cache_file.clone()
    }

    pub(in crate::native_app) fn has_loop_stable_playback_source(&self) -> bool {
        self.file.playback_samples.is_some() || self.file.playback_cache_file.is_some()
    }

    fn file_backed_playback_available(&self) -> bool {
        self.file.file_backed_playback_metadata_available()
    }
}
