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
        if self.file.path == old_path {
            Arc::make_mut(&mut self.file).path = new_path.to_path_buf();
            return true;
        }
        if let Ok(relative) = self.file.path.strip_prefix(old_path) {
            Arc::make_mut(&mut self.file).path = new_path.join(relative);
            return true;
        }
        false
    }

    pub(in crate::native_app) fn has_loaded_sample(&self) -> bool {
        !self.file.path.as_os_str().is_empty()
            && (!self.file.audio_bytes.is_empty()
                || self.file.playback_samples.is_some()
                || self.file.playback_cache_file.is_some()
                || self.file_backed_playback_available())
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

    fn file_backed_playback_available(&self) -> bool {
        self.file.audio_bytes.is_empty()
            && self.file.playback_samples.is_none()
            && self.file.playback_cache_file.is_none()
            && self.file.sample_rate != 0
            && self.file.channels != 0
            && self.file.frames != 0
            && self.file.path.is_file()
    }
}
