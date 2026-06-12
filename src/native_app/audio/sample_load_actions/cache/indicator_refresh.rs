use radiant::prelude as ui;
use std::{path::PathBuf, sync::Arc};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, WaveformCacheIndicatorRefreshResult},
    audio::sample_load_actions::cache::workers::probe_persisted_waveform_cache_indicators,
};

impl NativeAppState {
    #[cfg(test)]
    pub(in crate::native_app) fn refresh_persisted_waveform_cache_indicators(&mut self) {
        let audio_files = self
            .library
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

    pub(in crate::native_app) fn schedule_persisted_waveform_cache_indicator_refresh(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let paths = self
            .library
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| PathBuf::from(&file.id))
            .collect::<Vec<_>>();
        if paths.is_empty() {
            return;
        }
        for path in &paths {
            if self.waveform.cache.entries.contains_key(path) {
                self.waveform
                    .cache
                    .cached_sample_paths
                    .insert(path.display().to_string());
            }
        }
        let ticket = self.waveform.cache.indicator_refresh_task.begin();
        let results = Arc::clone(&self.waveform.cache.indicator_refresh_results);
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

    pub(in crate::native_app) fn finish_waveform_cache_indicator_refresh(
        &mut self,
        ticket: ui::TaskTicket,
    ) {
        let result = self
            .waveform
            .cache
            .indicator_refresh_results
            .lock()
            .ok()
            .and_then(|mut results| results.remove(&ticket));
        if !self.waveform.cache.indicator_refresh_task.finish(ticket) {
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
            if self.waveform.cache.entries.contains_key(&path)
                || result.playback_ready_paths.contains(&path)
            {
                self.waveform.cache.cached_sample_paths.insert(file_id);
            } else if result.warm_candidate_paths.contains(&path) {
                self.waveform.cache.cached_sample_paths.remove(&file_id);
                self.queue_waveform_cache_warm(path);
            } else {
                self.waveform.cache.cached_sample_paths.remove(&file_id);
            }
        }
    }
}
