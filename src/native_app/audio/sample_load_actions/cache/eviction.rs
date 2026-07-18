use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::native_app::{app::NativeAppState, waveform::invalidate_persisted_waveform_cache_path};

const WAVEFORM_MEMORY_CACHE_MAX_FILES: usize = 2048;
const WAVEFORM_MEMORY_CACHE_MAX_BYTES: usize = 2 * 1024 * 1024 * 1024;

impl NativeAppState {
    /// Release source-owned decoded/runtime cache state without touching reusable disk payloads.
    ///
    /// Durable playback payload retirement is driven asynchronously from the source database's
    /// reverse-ownership manifest after the lifecycle fence has joined old writers.
    pub(in crate::native_app) fn release_waveform_source_memory(&mut self, root: &Path) -> usize {
        self.waveform.cache.release_source_runtime(root)
    }

    pub(in crate::native_app) fn evict_waveform_cache_path(&mut self, path: &Path) {
        invalidate_persisted_waveform_cache_path(path);
        self.remove_waveform_cache_path(path);
        self.waveform.cache.order.retain(|cached| cached != path);
        self.waveform
            .cache
            .warm_pending
            .retain(|cached| cached != path);
        self.waveform
            .cache
            .cached_sample_paths
            .remove(&path.display().to_string());
        self.waveform
            .cache
            .instant_audition_sample_paths
            .remove(&path.display().to_string());
        self.waveform
            .cache
            .instant_audition_descriptors
            .remove(path);
    }

    pub(in crate::native_app) fn evict_waveform_cache_paths(&mut self, paths: &[PathBuf]) {
        if paths.len() <= 1 {
            if let Some(path) = paths.first() {
                self.evict_waveform_cache_path(path);
            }
            return;
        }

        crate::native_app::waveform::invalidate_persisted_waveform_cache_paths(paths);
        let path_set = paths.iter().collect::<HashSet<_>>();
        for path in paths {
            self.remove_waveform_cache_path(path);
            self.waveform
                .cache
                .cached_sample_paths
                .remove(&path.display().to_string());
            self.waveform
                .cache
                .instant_audition_sample_paths
                .remove(&path.display().to_string());
            self.waveform
                .cache
                .instant_audition_descriptors
                .remove(path);
        }
        self.waveform
            .cache
            .order
            .retain(|cached| !path_set.contains(cached));
        self.waveform
            .cache
            .warm_pending
            .retain(|cached| !path_set.contains(cached));
    }

    pub(super) fn enforce_waveform_cache_limit(&mut self) {
        while self.waveform.cache.order.len() > WAVEFORM_MEMORY_CACHE_MAX_FILES
            || (self.waveform.cache.bytes > WAVEFORM_MEMORY_CACHE_MAX_BYTES
                && self.waveform.cache.order.len() > 1)
        {
            let Some(path) = self.waveform.cache.order.pop_front() else {
                break;
            };
            if self.remove_waveform_cache_path(&path) {
                self.waveform
                    .cache
                    .cached_sample_paths
                    .remove(&path.display().to_string());
                self.waveform
                    .cache
                    .instant_audition_sample_paths
                    .remove(&path.display().to_string());
                self.waveform
                    .cache
                    .instant_audition_descriptors
                    .remove(&path);
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
}
