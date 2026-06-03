use std::{collections::hash_map::Entry, path::Path, sync::Arc, time::Instant};

use crate::gui_app::waveform::{
    WaveformFile, cached_waveform_file_exists, load_cached_waveform_file_for_playback,
};
use crate::gui_app::{GuiAppState, SampleFileSignature, WaveformCacheEntry, WaveformState};

use super::deferred_drop::defer_large_drop;

const WAVEFORM_MEMORY_CACHE_MAX_FILES: usize = 48;
const WAVEFORM_MEMORY_CACHE_MAX_BYTES: usize = 256 * 1024 * 1024;

impl GuiAppState {
    pub(super) fn cached_waveform_state(&mut self, path: &Path) -> Option<WaveformState> {
        let started_at = Instant::now();
        let path = path.to_path_buf();
        if let Some(file) = self.cached_memory_waveform_file(&path) {
            self.touch_waveform_cache_path(path.clone());
            log_slow_cache_phase("browser.sample_cache.lookup", &path, started_at);
            return Some(WaveformState::from_cached_file(file));
        }
        if let Some(file) = load_cached_waveform_file_for_playback(path.clone()).map(Arc::new) {
            let waveform = WaveformState::from_cached_file(file);
            self.remember_waveform(&waveform);
            log_slow_cache_phase("browser.sample_cache.lookup", &path, started_at);
            return Some(waveform);
        }
        self.cached_sample_paths.remove(&path.display().to_string());
        log_slow_cache_phase("browser.sample_cache.lookup", &path, started_at);
        None
    }

    fn cached_memory_waveform_file(&mut self, path: &Path) -> Option<Arc<WaveformFile>> {
        let entry = self.waveform_cache.get(path)?;
        let signature_started_at = Instant::now();
        let signature = sample_file_signature(path)?;
        log_slow_cache_phase("browser.sample_cache.signature", path, signature_started_at);
        if entry.signature != signature {
            self.remove_waveform_cache_path(path);
            self.cached_sample_paths.remove(&path.display().to_string());
            return None;
        }
        Some(Arc::clone(&entry.file))
    }

    pub(super) fn remember_waveform(&mut self, waveform: &WaveformState) {
        if !waveform.has_loaded_sample() {
            return;
        }
        let started_at = Instant::now();
        let path = waveform.path();
        let Some(signature) = sample_file_signature(&path) else {
            return;
        };
        let entry = WaveformCacheEntry {
            byte_len: waveform.audio_bytes().len()
                + waveform
                    .playback_samples()
                    .map(|samples| samples.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0),
            file: waveform.file(),
            signature,
        };
        self.insert_waveform_cache_entry(path, entry);
        log_slow_cache_phase(
            "browser.sample_cache.remember",
            &waveform.path(),
            started_at,
        );
    }

    pub(in crate::gui_app) fn refresh_persisted_waveform_cache_indicators(&mut self) {
        let audio_files = self
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        for file_id in audio_files {
            let path = std::path::PathBuf::from(&file_id);
            if self.waveform_cache.contains_key(&path) || cached_waveform_file_exists(&path) {
                self.cached_sample_paths.insert(file_id);
            } else {
                self.cached_sample_paths.remove(&file_id);
            }
        }
    }

    fn insert_waveform_cache_entry(&mut self, path: std::path::PathBuf, entry: WaveformCacheEntry) {
        match self.waveform_cache.entry(path.clone()) {
            Entry::Occupied(mut occupied) => {
                let old_entry = std::mem::replace(occupied.get_mut(), entry);
                self.waveform_cache_bytes = self
                    .waveform_cache_bytes
                    .saturating_sub(old_entry.byte_len)
                    .saturating_add(occupied.get().byte_len);
                defer_large_drop(old_entry);
            }
            Entry::Vacant(vacant) => {
                self.waveform_cache_bytes =
                    self.waveform_cache_bytes.saturating_add(entry.byte_len);
                vacant.insert(entry);
            }
        }
        self.cached_sample_paths.insert(path.display().to_string());
        self.touch_waveform_cache_path(path);
        self.enforce_waveform_cache_limit();
    }

    fn touch_waveform_cache_path(&mut self, path: std::path::PathBuf) {
        self.waveform_cache_order.retain(|cached| cached != &path);
        self.waveform_cache_order.push_back(path);
    }

    fn enforce_waveform_cache_limit(&mut self) {
        while self.waveform_cache_order.len() > WAVEFORM_MEMORY_CACHE_MAX_FILES
            || (self.waveform_cache_bytes > WAVEFORM_MEMORY_CACHE_MAX_BYTES
                && self.waveform_cache_order.len() > 1)
        {
            let Some(path) = self.waveform_cache_order.pop_front() else {
                break;
            };
            if self.remove_waveform_cache_path(&path) {
                self.remove_cached_sample_path_if_not_persisted(&path);
            }
        }
    }

    fn remove_waveform_cache_path(&mut self, path: &Path) -> bool {
        let Some(entry) = self.waveform_cache.remove(path) else {
            return false;
        };
        self.waveform_cache_bytes = self.waveform_cache_bytes.saturating_sub(entry.byte_len);
        defer_large_drop(entry);
        true
    }

    fn remove_cached_sample_path_if_not_persisted(&mut self, path: &Path) {
        if !cached_waveform_file_exists(path) {
            self.cached_sample_paths.remove(&path.display().to_string());
        }
    }
}

fn sample_file_signature(path: &Path) -> Option<SampleFileSignature> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified_ns = metadata
        .modified()
        .ok()?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .ok()?
        .as_nanos()
        .try_into()
        .ok()?;
    Some(SampleFileSignature {
        size_bytes: metadata.len(),
        modified_ns,
    })
}

fn log_slow_cache_phase(event: &'static str, path: &Path, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < std::time::Duration::from_millis(4) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        path = %path.display(),
        "Slow sample cache phase"
    );
}
