use radiant::prelude as ui;
use std::{
    collections::{HashSet, hash_map::Entry},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::gui_app::waveform::{
    cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
    load_cached_waveform_file_for_playback,
};
use crate::gui_app::{
    ActiveFolderCacheWarmResult, GuiAppState, GuiMessage, WaveformCacheEntry,
    WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult, WaveformState,
};

const WAVEFORM_MEMORY_CACHE_MAX_FILES: usize = 48;
const WAVEFORM_MEMORY_CACHE_MAX_BYTES: usize = 2 * 1024 * 1024 * 1024;
const WAVEFORM_CACHE_WARM_BATCH_MAX_FILES: usize = 8;
const ACTIVE_FOLDER_CACHE_WARM_DELAY: Duration = Duration::from_millis(750);
const ACTIVE_FOLDER_CACHE_WARM_BATCH_MAX_FILES: usize = 4;

pub(in crate::gui_app) fn active_folder_cache_warm_priority() -> ui::TaskPriority {
    ui::TaskPriority::Idle
}

impl GuiAppState {
    pub(in crate::gui_app) fn remember_waveform(&mut self, waveform: &WaveformState) {
        if !waveform.has_loaded_sample() {
            return;
        }
        let started_at = Instant::now();
        let file = waveform.file();
        let entry = WaveformCacheEntry {
            byte_len: waveform.audio_bytes().len()
                + waveform
                    .playback_samples()
                    .map(|samples| samples.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0),
            file,
        };
        self.insert_waveform_cache_entry(waveform.path(), entry);
        log_slow_cache_phase(
            "browser.sample_cache.remember",
            &waveform.path(),
            started_at,
        );
    }

    pub(in crate::gui_app) fn remap_renamed_waveform_cache_path(
        &mut self,
        old_path: &Path,
        new_path: &Path,
    ) {
        let cache_paths = self.waveform_cache.keys().cloned().collect::<Vec<_>>();
        for path in cache_paths {
            let Some(mapped) = remapped_cache_path(&path, old_path, new_path) else {
                continue;
            };
            if mapped == path {
                continue;
            }
            if let Some(entry) = self.waveform_cache.remove(&path) {
                self.waveform_cache.insert(mapped, entry);
            }
        }

        self.waveform_cache_order = self
            .waveform_cache_order
            .iter()
            .map(|path| {
                remapped_cache_path(path, old_path, new_path).unwrap_or_else(|| path.clone())
            })
            .collect();
        self.waveform_cache_warm_pending = self
            .waveform_cache_warm_pending
            .iter()
            .map(|path| {
                remapped_cache_path(path, old_path, new_path).unwrap_or_else(|| path.clone())
            })
            .collect();
        self.cached_sample_paths = self
            .cached_sample_paths
            .iter()
            .map(|id| {
                let path = PathBuf::from(id);
                remapped_cache_path(&path, old_path, new_path)
                    .map(|mapped| mapped.display().to_string())
                    .unwrap_or_else(|| id.clone())
            })
            .collect();
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn refresh_persisted_waveform_cache_indicators(&mut self) {
        let audio_files = self
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        let result = probe_persisted_waveform_cache_indicators(
            audio_files.into_iter().map(PathBuf::from).collect(),
        );
        self.apply_waveform_cache_indicator_refresh_result(result);
    }

    pub(in crate::gui_app) fn schedule_persisted_waveform_cache_indicator_refresh(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let paths = self
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| PathBuf::from(&file.id))
            .collect::<Vec<_>>();
        if paths.is_empty() {
            return;
        }
        for path in &paths {
            if self.waveform_cache.contains_key(path) {
                self.cached_sample_paths.insert(path.display().to_string());
            }
        }
        let ticket = self.waveform_cache_indicator_refresh_task.begin();
        let results = Arc::clone(&self.waveform_cache_indicator_refresh_results);
        context.spawn(
            "gui-waveform-cache-indicators",
            move || {
                let result = probe_persisted_waveform_cache_indicators(paths);
                if let Ok(mut results) = results.lock() {
                    results.insert(ticket, result);
                }
                ticket
            },
            GuiMessage::WaveformCacheIndicatorRefreshFinished,
        );
    }

    pub(in crate::gui_app) fn schedule_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.cancel_active_folder_cache_warm();
        let Some((folder_id, paths)) = self.folder_browser.selected_folder_cache_warm_request()
        else {
            return;
        };
        if paths.is_empty() {
            return;
        }
        self.active_folder_cache_warm_folder_id = Some(folder_id);
        self.active_folder_cache_warm_pending = paths.into();
        context.after_latest(
            &mut self.active_folder_cache_warm_delay_task,
            ACTIVE_FOLDER_CACHE_WARM_DELAY,
            GuiMessage::ActiveFolderCacheWarmReady,
        );
    }

    pub(in crate::gui_app) fn cancel_active_folder_cache_warm(&mut self) {
        self.active_folder_cache_warm_delay_task.cancel();
        self.active_folder_cache_warm_task.cancel();
        if let Some(token) = self.active_folder_cache_warm_cancel.take() {
            token.cancel();
        }
        self.active_folder_cache_warm_folder_id = None;
        self.active_folder_cache_warm_pending.clear();
    }

    pub(in crate::gui_app) fn start_active_folder_cache_warm_after_delay(
        &mut self,
        ticket: ui::TaskTicket,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if !self.active_folder_cache_warm_delay_task.finish(ticket) {
            return;
        }
        self.maybe_start_active_folder_cache_warm(context);
    }

    pub(in crate::gui_app) fn finish_waveform_cache_indicator_refresh(
        &mut self,
        ticket: ui::TaskTicket,
    ) {
        let result = self
            .waveform_cache_indicator_refresh_results
            .lock()
            .ok()
            .and_then(|mut results| results.remove(&ticket));
        if !self.waveform_cache_indicator_refresh_task.finish(ticket) {
            return;
        }
        if let Some(result) = result {
            self.apply_waveform_cache_indicator_refresh_result(result);
        }
    }

    fn apply_waveform_cache_indicator_refresh_result(
        &mut self,
        result: WaveformCacheIndicatorRefreshResult,
    ) {
        for path in result.probed_paths {
            let file_id = path.display().to_string();
            if self.waveform_cache.contains_key(&path)
                || result.playback_ready_paths.contains(&path)
            {
                self.cached_sample_paths.insert(file_id);
            } else if result.warm_candidate_paths.contains(&path) {
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

    pub(in crate::gui_app) fn maybe_start_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if self.active_folder_cache_warm_delay_task.active().is_some()
            || self.active_folder_cache_warm_task.active().is_some()
        {
            return;
        }
        let Some(folder_id) = self.active_folder_cache_warm_folder_id.clone() else {
            return;
        };
        let paths = self.next_active_folder_cache_warm_batch();
        if paths.is_empty() {
            self.active_folder_cache_warm_folder_id = None;
            return;
        }
        self.active_folder_cache_warm_cancel =
            Some(context.spawn_cancellable_latest_with_priority(
                &mut self.active_folder_cache_warm_task,
                "gui-active-folder-cache-warm",
                active_folder_cache_warm_priority(),
                move |_ticket, token| {
                    let loaded = warm_active_folder_waveform_cache(paths, &token);
                    ActiveFolderCacheWarmResult {
                        folder_id,
                        loaded,
                        cancelled: token.is_cancelled(),
                    }
                },
                GuiMessage::ActiveFolderCacheWarmFinished,
            ));
    }

    pub(in crate::gui_app) fn finish_active_folder_cache_warm(
        &mut self,
        completion: ui::TaskCompletion<ActiveFolderCacheWarmResult>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self.active_folder_cache_warm_task.finish(completion.ticket) {
            return;
        }
        self.active_folder_cache_warm_cancel = None;
        let result = completion.output;
        if self.active_folder_cache_warm_folder_id.as_deref() != Some(result.folder_id.as_str()) {
            return;
        }
        for (_path, file) in result.loaded {
            let waveform = WaveformState::from_cached_file(file);
            self.remember_waveform(&waveform);
        }
        if result.cancelled {
            return;
        }
        log_slow_cache_phase(
            "browser.sample_cache.active_folder_finish",
            Path::new(&result.folder_id),
            started_at,
        );
        self.maybe_start_active_folder_cache_warm(context);
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

    fn next_active_folder_cache_warm_batch(&mut self) -> Vec<PathBuf> {
        let mut batch = Vec::new();
        while batch.len() < ACTIVE_FOLDER_CACHE_WARM_BATCH_MAX_FILES {
            let Some(path) = self.active_folder_cache_warm_pending.pop_front() else {
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
        self.touch_cached_waveform_path(path);
        self.enforce_waveform_cache_limit();
    }

    pub(super) fn touch_cached_waveform_path(&mut self, path: std::path::PathBuf) {
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

fn remapped_cache_path(path: &Path, old_path: &Path, new_path: &Path) -> Option<PathBuf> {
    if path == old_path {
        return Some(new_path.to_path_buf());
    }
    path.strip_prefix(old_path)
        .ok()
        .map(|relative| new_path.join(relative))
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

pub(in crate::gui_app) fn warm_active_folder_waveform_cache(
    paths: Vec<PathBuf>,
    token: &ui::CancellationToken,
) -> Vec<(PathBuf, Arc<crate::gui_app::waveform::WaveformFile>)> {
    paths
        .into_iter()
        .filter_map(|path| {
            if token.is_cancelled() {
                return None;
            }
            if let Some(file) = load_cached_waveform_file_for_playback(path.clone()) {
                return Some((path, Arc::new(file)));
            }
            let waveform = WaveformState::load_path_with_progress_and_cancel(
                path.clone(),
                |_| {},
                || token.is_cancelled(),
            )
            .ok()?;
            Some((path, waveform.file()))
        })
        .collect()
}

fn probe_persisted_waveform_cache_indicators(
    paths: Vec<PathBuf>,
) -> WaveformCacheIndicatorRefreshResult {
    let mut playback_ready_paths = HashSet::new();
    let mut warm_candidate_paths = HashSet::new();
    for path in &paths {
        if cached_waveform_file_playback_ready_exists(path) {
            playback_ready_paths.insert(path.clone());
        } else if cached_waveform_file_exists(path) {
            warm_candidate_paths.insert(path.clone());
        }
    }
    WaveformCacheIndicatorRefreshResult {
        probed_paths: paths,
        playback_ready_paths,
        warm_candidate_paths,
    }
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
