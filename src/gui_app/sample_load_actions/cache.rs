use radiant::prelude as ui;
use std::{collections::hash_map::Entry, path::Path, path::PathBuf, sync::Arc, time::Instant};

use crate::gui_app::waveform::{
    cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
    load_cached_waveform_file_for_playback,
};
use crate::gui_app::{
    GuiAppState, GuiMessage, WaveformCacheEntry, WaveformCacheWarmResult, WaveformState,
};

const WAVEFORM_MEMORY_CACHE_MAX_FILES: usize = 48;
const WAVEFORM_MEMORY_CACHE_MAX_BYTES: usize = 2 * 1024 * 1024 * 1024;
const WAVEFORM_CACHE_WARM_BATCH_MAX_FILES: usize = 8;

impl GuiAppState {
    pub(super) fn remember_waveform(&mut self, waveform: &WaveformState) {
        if !waveform.has_loaded_sample() {
            return;
        }
        let started_at = Instant::now();
        let entry = WaveformCacheEntry {
            byte_len: waveform.audio_bytes().len()
                + waveform
                    .playback_samples()
                    .map(|samples| samples.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0),
        };
        self.insert_waveform_cache_entry(waveform.path(), entry);
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
            let path = PathBuf::from(&file_id);
            if self.waveform_cache.contains_key(&path) {
                self.cached_sample_paths.insert(file_id);
            } else if cached_waveform_file_playback_ready_exists(&path) {
                self.cached_sample_paths.insert(file_id);
            } else if cached_waveform_file_exists(&path) {
                self.cached_sample_paths.remove(&file_id);
                self.queue_waveform_cache_warm(path);
            } else {
                self.cached_sample_paths.remove(&file_id);
            }
        }
    }

    pub(in crate::gui_app) fn maybe_start_waveform_cache_warm(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if self.waveform_cache_warm_task.active().is_some() {
            return;
        }
        let paths = self.next_waveform_cache_warm_batch();
        if paths.is_empty() {
            return;
        }
        let ticket = self.waveform_cache_warm_task.begin();
        let results = Arc::clone(&self.waveform_cache_warm_results);
        context.spawn(
            "gui-waveform-cache-warm",
            move || {
                let result = warm_persisted_waveform_cache(paths);
                if let Ok(mut results) = results.lock() {
                    results.insert(ticket, result);
                }
                ticket
            },
            GuiMessage::WaveformCacheWarmFinished,
        );
    }

    pub(in crate::gui_app) fn finish_waveform_cache_warm(&mut self, ticket: ui::TaskTicket) {
        let started_at = Instant::now();
        let result = self
            .waveform_cache_warm_results
            .lock()
            .ok()
            .and_then(|mut results| results.remove(&ticket));
        if !self.waveform_cache_warm_task.finish(ticket) {
            return;
        }
        if let Some(result) = result {
            self.apply_waveform_cache_warm_result(result);
        }
        log_slow_cache_phase(
            "browser.sample_cache.warm_finish",
            Path::new("waveform-cache-warm"),
            started_at,
        );
    }

    pub(in crate::gui_app) fn apply_waveform_cache_warm_result(
        &mut self,
        result: WaveformCacheWarmResult,
    ) {
        for (_path, file) in result.loaded {
            let waveform = WaveformState::from_cached_file(file);
            self.remember_waveform(&waveform);
        }
    }

    fn queue_waveform_cache_warm(&mut self, path: PathBuf) {
        if self.waveform_cache.contains_key(&path)
            || self
                .waveform_cache_warm_pending
                .iter()
                .any(|queued| queued == &path)
        {
            return;
        }
        self.waveform_cache_warm_pending.push_back(path);
    }

    fn next_waveform_cache_warm_batch(&mut self) -> Vec<PathBuf> {
        let mut batch = Vec::new();
        while batch.len() < WAVEFORM_CACHE_WARM_BATCH_MAX_FILES {
            let Some(path) = self.waveform_cache_warm_pending.pop_front() else {
                break;
            };
            if self.waveform_cache.contains_key(&path) || batch.iter().any(|queued| queued == &path)
            {
                continue;
            }
            batch.push(path);
        }
        batch
    }

    fn insert_waveform_cache_entry(&mut self, path: PathBuf, entry: WaveformCacheEntry) {
        match self.waveform_cache.entry(path.clone()) {
            Entry::Occupied(mut occupied) => {
                let old_entry = std::mem::replace(occupied.get_mut(), entry);
                self.waveform_cache_bytes = self
                    .waveform_cache_bytes
                    .saturating_sub(old_entry.byte_len)
                    .saturating_add(occupied.get().byte_len);
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
        true
    }

    fn remove_cached_sample_path_if_not_persisted(&mut self, path: &Path) {
        if !cached_waveform_file_playback_ready_exists(path) {
            self.cached_sample_paths.remove(&path.display().to_string());
        }
    }
}

pub(in crate::gui_app) fn warm_persisted_waveform_cache(
    paths: Vec<PathBuf>,
) -> WaveformCacheWarmResult {
    let loaded = paths
        .into_iter()
        .filter_map(|path| {
            load_cached_waveform_file_for_playback(path.clone())
                .map(Arc::new)
                .map(|file| (path, file))
        })
        .collect();
    WaveformCacheWarmResult { loaded }
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
