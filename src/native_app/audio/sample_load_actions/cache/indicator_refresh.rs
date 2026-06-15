use radiant::prelude as ui;

use crate::native_app::{
    app::{GuiMessage, NativeAppState, WaveformCacheIndicatorRefreshResult},
    audio::sample_load_actions::cache::{
        WAVEFORM_CACHE_INDICATOR_REFRESH_MAX_FILES,
        workers::probe_persisted_waveform_cache_indicators,
    },
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
            audio_files
                .into_iter()
                .map(std::path::PathBuf::from)
                .collect(),
            || false,
        );
        self.apply_waveform_cache_indicator_refresh_result(result);
    }

    pub(in crate::native_app) fn schedule_persisted_waveform_cache_indicator_refresh(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let paths = self
            .library
            .folder_browser
            .selected_cache_candidate_paths(WAVEFORM_CACHE_INDICATOR_REFRESH_MAX_FILES);
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
        context
            .business()
            .background("gui-waveform-cache-indicators")
            .cancellable()
            .latest(&mut self.waveform.cache.indicator_refresh_task)
            .run(
                move |context| {
                    probe_persisted_waveform_cache_indicators(paths, || context.is_cancelled())
                },
                GuiMessage::WaveformCacheIndicatorRefreshFinished,
            );
    }

    pub(in crate::native_app) fn finish_waveform_cache_indicator_refresh(
        &mut self,
        completion: ui::TaskCompletion<WaveformCacheIndicatorRefreshResult>,
    ) {
        if !self
            .waveform
            .cache
            .indicator_refresh_task
            .finish(completion.ticket)
        {
            return;
        }
        self.apply_waveform_cache_indicator_refresh_result(completion.output);
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
