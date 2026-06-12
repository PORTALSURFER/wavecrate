use radiant::prelude as ui;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, WaveformCacheWarmResult, WaveformState},
    audio::sample_load_actions::cache::{
        WAVEFORM_CACHE_WARM_BATCH_MAX_FILES, logging::log_slow_cache_phase,
        workers::warm_persisted_waveform_cache,
    },
};

impl NativeAppState {
    pub(in crate::native_app) fn maybe_start_waveform_cache_warm(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if self.waveform.cache.warm_task.active().is_some() {
            return;
        }
        let paths = self.next_waveform_cache_warm_batch();
        if paths.is_empty() {
            return;
        }
        let ticket = self.waveform.cache.warm_task.begin();
        let results = Arc::clone(&self.waveform.cache.warm_results);
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

    pub(in crate::native_app) fn finish_waveform_cache_warm(&mut self, ticket: ui::TaskTicket) {
        let started_at = Instant::now();
        let result = self
            .waveform
            .cache
            .warm_results
            .lock()
            .ok()
            .and_then(|mut results| results.remove(&ticket));
        if !self.waveform.cache.warm_task.finish(ticket) {
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

    pub(in crate::native_app) fn apply_waveform_cache_warm_result(
        &mut self,
        result: WaveformCacheWarmResult,
    ) {
        for (_path, file) in result.loaded {
            let waveform = WaveformState::from_cached_file(file);
            self.remember_waveform(&waveform);
        }
    }

    pub(super) fn queue_waveform_cache_warm(&mut self, path: PathBuf) {
        if self.waveform.cache.entries.contains_key(&path)
            || self
                .waveform
                .cache
                .warm_pending
                .iter()
                .any(|queued| queued == &path)
        {
            return;
        }
        self.waveform.cache.warm_pending.push_back(path);
    }

    fn next_waveform_cache_warm_batch(&mut self) -> Vec<PathBuf> {
        let entries = &self.waveform.cache.entries;
        take_cache_warm_batch(
            &mut self.waveform.cache.warm_pending,
            |path| entries.contains_key(path),
            WAVEFORM_CACHE_WARM_BATCH_MAX_FILES,
        )
    }
}

pub(super) fn take_cache_warm_batch<F>(
    pending: &mut VecDeque<PathBuf>,
    is_cached: F,
    max_files: usize,
) -> Vec<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    let mut batch = Vec::new();
    while batch.len() < max_files {
        let Some(path) = pending.pop_front() else {
            break;
        };
        if is_cached(&path) || batch.iter().any(|queued| queued == &path) {
            continue;
        }
        batch.push(path);
    }
    batch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_cache_warm_batch_skips_cached_paths_and_batch_duplicates() {
        let cached_path = PathBuf::from("cached.wav");
        let fresh_path = PathBuf::from("fresh.wav");
        let duplicate_path = PathBuf::from("duplicate.wav");
        let mut pending = VecDeque::from([
            cached_path.clone(),
            fresh_path.clone(),
            duplicate_path.clone(),
            duplicate_path.clone(),
        ]);

        let batch = take_cache_warm_batch(&mut pending, |path| path == cached_path, 8);

        assert_eq!(batch, vec![fresh_path, duplicate_path]);
        assert!(pending.is_empty());
    }

    #[test]
    fn take_cache_warm_batch_leaves_later_paths_pending_after_limit() {
        let first = PathBuf::from("first.wav");
        let second = PathBuf::from("second.wav");
        let mut pending = VecDeque::from([first.clone(), second.clone()]);

        let batch = take_cache_warm_batch(&mut pending, |_| false, 1);

        assert_eq!(batch, vec![first]);
        assert_eq!(pending, VecDeque::from([second]));
    }
}
