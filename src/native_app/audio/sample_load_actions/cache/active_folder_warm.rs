use radiant::prelude as ui;
use std::{path::Path, time::Instant};

use crate::native_app::{
    app::{ActiveFolderCacheWarmResult, GuiMessage, NativeAppState, WaveformState},
    audio::sample_load_actions::{
        active_folder_cache_warm_resource_key,
        cache::{
            ACTIVE_FOLDER_CACHE_WARM_BATCH_MAX_FILES, ACTIVE_FOLDER_CACHE_WARM_DELAY,
            ACTIVE_FOLDER_CACHE_WARM_MAX_PENDING_FILES, active_folder_cache_warm_priority,
            logging::log_slow_cache_phase, persisted_warm::take_cache_warm_batch,
            workers::warm_active_folder_waveform_cache,
        },
    },
};

impl NativeAppState {
    pub(in crate::native_app) fn schedule_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.cancel_active_folder_cache_warm();
        let Some((folder_id, paths)) = self
            .library
            .folder_browser
            .selected_folder_cache_warm_request(ACTIVE_FOLDER_CACHE_WARM_MAX_PENDING_FILES)
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
        if let Some(key) = self.waveform.cache.active_folder_warm_key.take() {
            self.waveform.cache.active_folder_warm_tasks.cancel(&key);
        }
        if let Some(token) = self.waveform.cache.active_folder_warm_cancel.take() {
            token.cancel();
        }
        self.waveform.cache.active_folder_warm_folder_id = None;
        self.waveform.cache.active_folder_warm_pending.clear();
    }

    pub(in crate::native_app) fn start_active_folder_cache_warm_after_delay(
        &mut self,
        ticket: ui::TaskTicket,
        context: &mut ui::UiUpdateContext<GuiMessage>,
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
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.sample_cache_warm_should_yield() {
            self.pause_active_folder_cache_warm(context);
            return;
        }
        if self
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some()
            || self
                .waveform
                .cache
                .active_folder_warm_key
                .as_ref()
                .and_then(|key| self.waveform.cache.active_folder_warm_tasks.active(key))
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
        let key = active_folder_cache_warm_resource_key(&folder_id);
        let Some(warm) = (match active_folder_cache_warm_priority() {
            ui::TaskPriority::Interactive => context
                .business()
                .interactive("gui-active-folder-cache-warm"),
            ui::TaskPriority::Background => context
                .business()
                .background("gui-active-folder-cache-warm"),
            ui::TaskPriority::Idle => context.business().idle("gui-active-folder-cache-warm"),
        })
        .cancellable()
        .exclusive_for(
            &mut self.waveform.cache.active_folder_warm_tasks,
            key.clone(),
        ) else {
            return;
        };
        self.waveform.cache.active_folder_warm_key = Some(key);
        self.waveform.cache.active_folder_warm_cancel = Some(warm.run(
            move |worker_context| {
                let loaded =
                    warm_active_folder_waveform_cache(paths, || worker_context.is_cancelled());
                ActiveFolderCacheWarmResult {
                    folder_id,
                    loaded,
                    cancelled: worker_context.is_cancelled(),
                }
            },
            GuiMessage::ActiveFolderCacheWarmFinished,
        ));
    }

    pub(in crate::native_app) fn pause_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if let Some(token) = self.waveform.cache.active_folder_warm_cancel.take() {
            token.cancel();
        }
        if let Some(key) = self.waveform.cache.active_folder_warm_key.take() {
            self.waveform.cache.active_folder_warm_tasks.cancel(&key);
        }
        self.reschedule_active_folder_cache_warm_delay(context);
    }

    fn reschedule_active_folder_cache_warm_delay(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.waveform.cache.active_folder_warm_folder_id.is_none()
            || self.waveform.cache.active_folder_warm_pending.is_empty()
            || self
                .waveform
                .cache
                .active_folder_warm_delay_task
                .active()
                .is_some()
        {
            return;
        }
        context.after_latest(
            &mut self.waveform.cache.active_folder_warm_delay_task,
            ACTIVE_FOLDER_CACHE_WARM_DELAY,
            GuiMessage::ActiveFolderCacheWarmReady,
        );
    }

    pub(in crate::native_app) fn finish_active_folder_cache_warm(
        &mut self,
        completion: ui::KeyedTaskCompletion<ui::ResourceKey, ActiveFolderCacheWarmResult>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self
            .waveform
            .cache
            .active_folder_warm_tasks
            .finish_key(&completion.key, completion.ticket)
        {
            return;
        }
        self.waveform.cache.active_folder_warm_key = None;
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
        if self.sample_cache_warm_should_yield() {
            self.reschedule_active_folder_cache_warm_delay(context);
        } else {
            self.maybe_start_active_folder_cache_warm(context);
        }
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
