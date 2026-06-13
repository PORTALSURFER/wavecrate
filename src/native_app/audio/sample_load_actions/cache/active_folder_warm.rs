use radiant::prelude as ui;
use std::{path::Path, time::Instant};

use crate::native_app::{
    app::{ActiveFolderCacheWarmResult, GuiMessage, NativeAppState, WaveformState},
    audio::sample_load_actions::cache::{
        ACTIVE_FOLDER_CACHE_WARM_BATCH_MAX_FILES, ACTIVE_FOLDER_CACHE_WARM_DELAY,
        active_folder_cache_warm_priority, logging::log_slow_cache_phase,
        persisted_warm::take_cache_warm_batch, workers::warm_active_folder_waveform_cache,
    },
};

impl NativeAppState {
    pub(in crate::native_app) fn schedule_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.cancel_active_folder_cache_warm();
        let Some((folder_id, paths)) = self
            .library
            .folder_browser
            .selected_folder_cache_warm_request()
        else {
            return;
        };
        if paths.is_empty() {
            return;
        }
        self.waveform.cache.active_folder_warm_folder_id = Some(folder_id);
        self.waveform.cache.active_folder_warm_pending = paths.into();
        context.after_latest(
            &mut self.waveform.cache.active_folder_warm_delay_task,
            ACTIVE_FOLDER_CACHE_WARM_DELAY,
            GuiMessage::ActiveFolderCacheWarmReady,
        );
    }

    pub(in crate::native_app) fn cancel_active_folder_cache_warm(&mut self) {
        self.waveform.cache.active_folder_warm_delay_task.cancel();
        self.waveform.cache.active_folder_warm_task.cancel();
        if let Some(token) = self.waveform.cache.active_folder_warm_cancel.take() {
            token.cancel();
        }
        self.waveform.cache.active_folder_warm_folder_id = None;
        self.waveform.cache.active_folder_warm_pending.clear();
    }

    pub(in crate::native_app) fn start_active_folder_cache_warm_after_delay(
        &mut self,
        ticket: ui::TaskTicket,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if !self
            .waveform
            .cache
            .active_folder_warm_delay_task
            .finish(ticket)
        {
            return;
        }
        self.maybe_start_active_folder_cache_warm(context);
    }

    pub(in crate::native_app) fn maybe_start_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if self
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some()
            || self
                .waveform
                .cache
                .active_folder_warm_task
                .active()
                .is_some()
        {
            return;
        }
        let Some(folder_id) = self.waveform.cache.active_folder_warm_folder_id.clone() else {
            return;
        };
        let paths = self.next_active_folder_cache_warm_batch();
        if paths.is_empty() {
            self.waveform.cache.active_folder_warm_folder_id = None;
            return;
        }
        let warm = match active_folder_cache_warm_priority() {
            ui::TaskPriority::Interactive => context
                .business()
                .interactive("gui-active-folder-cache-warm"),
            ui::TaskPriority::Background => context
                .business()
                .background("gui-active-folder-cache-warm"),
            ui::TaskPriority::Idle => context.business().idle("gui-active-folder-cache-warm"),
        }
        .cancellable()
        .latest(&mut self.waveform.cache.active_folder_warm_task);
        self.waveform.cache.active_folder_warm_cancel = Some(warm.run(
            move |worker_context| {
                let loaded = warm_active_folder_waveform_cache(paths, &worker_context);
                ActiveFolderCacheWarmResult {
                    folder_id,
                    loaded,
                    cancelled: worker_context.is_cancelled(),
                }
            },
            GuiMessage::ActiveFolderCacheWarmFinished,
        ));
    }

    pub(in crate::native_app) fn finish_active_folder_cache_warm(
        &mut self,
        completion: ui::TaskCompletion<ActiveFolderCacheWarmResult>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self
            .waveform
            .cache
            .active_folder_warm_task
            .finish(completion.ticket)
        {
            return;
        }
        self.waveform.cache.active_folder_warm_cancel = None;
        let result = completion.output;
        if self.waveform.cache.active_folder_warm_folder_id.as_deref()
            != Some(result.folder_id.as_str())
        {
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

    fn next_active_folder_cache_warm_batch(&mut self) -> Vec<std::path::PathBuf> {
        let entries = &self.waveform.cache.entries;
        take_cache_warm_batch(
            &mut self.waveform.cache.active_folder_warm_pending,
            |path| entries.contains_key(path),
            ACTIVE_FOLDER_CACHE_WARM_BATCH_MAX_FILES,
        )
    }
}
