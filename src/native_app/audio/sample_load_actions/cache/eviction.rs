use std::path::Path;

use crate::native_app::{
    app::NativeAppState, waveform::cached_waveform_file_playback_ready_exists,
};

const WAVEFORM_MEMORY_CACHE_MAX_FILES: usize = 48;
const WAVEFORM_MEMORY_CACHE_MAX_BYTES: usize = 2 * 1024 * 1024 * 1024;

impl NativeAppState {
    pub(super) fn enforce_waveform_cache_limit(&mut self) {
        while self.waveform.cache.order.len() > WAVEFORM_MEMORY_CACHE_MAX_FILES
            || (self.waveform.cache.bytes > WAVEFORM_MEMORY_CACHE_MAX_BYTES
                && self.waveform.cache.order.len() > 1)
        {
            let Some(path) = self.waveform.cache.order.pop_front() else {
                break;
            };
            if self.remove_waveform_cache_path(&path) {
                self.remove_cached_sample_path_if_not_persisted(&path);
            }
        }
    }

    fn remove_waveform_cache_path(&mut self, path: &Path) -> bool {
        let Some(entry) = self.waveform.cache.entries.remove(path) else {
            return false;
        };
        self.waveform.cache.bytes = self.waveform.cache.bytes.saturating_sub(entry.byte_len);
        true
    }

    fn remove_cached_sample_path_if_not_persisted(&mut self, path: &Path) {
        if !cached_waveform_file_playback_ready_exists(path) {
            self.waveform
                .cache
                .cached_sample_paths
                .remove(&path.display().to_string());
        }
    }
}
