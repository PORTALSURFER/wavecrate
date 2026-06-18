use radiant::prelude as ui;
use radiant::runtime::{BusinessEventSink, BusinessWorkContext};
use std::{
    path::Path,
    time::{Duration, Instant},
};

use crate::native_app::{
    app::{
        ActiveFolderCacheWarmPlanResult, ActiveFolderCacheWarmProgress,
        ActiveFolderCacheWarmResult, GuiMessage, NativeAppState, WaveformState,
    },
    audio::sample_load_actions::{
        active_folder_cache_warm_resource_key,
        cache::{
            ACTIVE_FOLDER_CACHE_WARM_CONTINUATION_DELAY, ACTIVE_FOLDER_CACHE_WARM_INITIAL_DELAY,
            ACTIVE_FOLDER_CACHE_WARM_LIGHT_CONTINUATION_DELAY,
            ACTIVE_FOLDER_CACHE_WARM_SCAN_MAX_FILES, active_folder_cache_warm_priority,
            logging::log_slow_cache_phase,
            persisted_warm::take_cache_warm_batch,
            workers::{
                plan_active_folder_waveform_cache_warm,
                warm_active_folder_waveform_cache_with_progress,
            },
        },
    },
};

const ACTIVE_FOLDER_CACHE_PROGRESS_YIELD_INTERVAL: Duration = Duration::from_millis(8);

impl NativeAppState {
    pub(in crate::native_app) fn schedule_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.cancel_active_folder_cache_warm();
        let Some((folder_id, paths)) = self
            .library
            .folder_browser
            .selected_source_cache_warm_request()
        else {
            return;
        };
        if paths.is_empty() {
            return;
        }
        context
            .business()
            .background("gui-active-folder-cache-warm-plan")
            .cancellable()
            .latest(&mut self.waveform.cache.active_folder_warm_plan_task)
            .run(
                move |worker_context| {
                    plan_active_folder_waveform_cache_warm(folder_id, paths, || {
                        worker_context.is_cancelled()
                    })
                },
                GuiMessage::ActiveFolderCacheWarmPlanned,
            );
    }

    pub(in crate::native_app) fn cancel_active_folder_cache_warm(&mut self) {
        self.waveform.cache.active_folder_warm_plan_task.cancel();
        self.waveform.cache.active_folder_warm_delay_task.cancel();
        if let Some(key) = self.waveform.cache.active_folder_warm_key.take() {
            self.waveform.cache.active_folder_warm_tasks.cancel(&key);
        }
        if let Some(token) = self.waveform.cache.active_folder_warm_cancel.take() {
            token.cancel();
        }
        self.waveform.cache.active_folder_warm_folder_id = None;
        self.waveform.cache.active_folder_warm_pending.clear();
        self.waveform.cache.active_folder_warm_completed = 0;
        self.waveform.cache.active_folder_warm_total = 0;
        self.waveform.cache.active_folder_warm_current = None;
        self.waveform.cache.active_folder_warm_current_progress = 0.0;
        self.waveform.cache.active_folder_warm_current_stage = None;
        self.waveform.cache.active_folder_warm_batch_base_completed = 0;
    }

    pub(in crate::native_app) fn finish_active_folder_cache_warm_plan(
        &mut self,
        completion: ui::TaskCompletion<ActiveFolderCacheWarmPlanResult>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self
            .waveform
            .cache
            .active_folder_warm_plan_task
            .finish(completion.ticket)
        {
            return;
        }
        let result = completion.output;
        if result.cancelled {
            return;
        }
        for path in result.playback_ready {
            self.waveform
                .cache
                .cached_sample_paths
                .insert(path.display().to_string());
        }
        if result.pending.is_empty() {
            return;
        }
        let total = result.pending.len();
        self.waveform.cache.active_folder_warm_folder_id = Some(result.folder_id);
        self.waveform.cache.active_folder_warm_pending = result.pending.into();
        self.waveform.cache.active_folder_warm_completed = 0;
        self.waveform.cache.active_folder_warm_total = total;
        self.waveform.cache.active_folder_warm_current = None;
        self.waveform.cache.active_folder_warm_current_progress = 0.0;
        self.waveform.cache.active_folder_warm_current_stage = None;
        self.waveform.cache.active_folder_warm_batch_base_completed = 0;
        context.after_latest(
            &mut self.waveform.cache.active_folder_warm_delay_task,
            ACTIVE_FOLDER_CACHE_WARM_INITIAL_DELAY,
            GuiMessage::ActiveFolderCacheWarmReady,
        );
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
        if self
            .waveform
            .cache
            .active_folder_warm_key
            .as_ref()
            .and_then(|key| self.waveform.cache.active_folder_warm_tasks.active(key))
            .is_some()
        {
            if self.sample_cache_warm_should_pause_active() {
                self.pause_active_folder_cache_warm(context);
            }
            return;
        }
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
        {
            return;
        }
        let Some(folder_id) = self.waveform.cache.active_folder_warm_folder_id.clone() else {
            return;
        };
        let paths = self.next_active_folder_cache_warm_batch();
        if paths.is_empty() {
            self.waveform.cache.active_folder_warm_folder_id = None;
            self.waveform.cache.active_folder_warm_completed = 0;
            self.waveform.cache.active_folder_warm_total = 0;
            self.waveform.cache.active_folder_warm_current = None;
            self.waveform.cache.active_folder_warm_current_progress = 0.0;
            self.waveform.cache.active_folder_warm_current_stage = None;
            self.waveform.cache.active_folder_warm_batch_base_completed = 0;
            return;
        }
        let key = active_folder_cache_warm_resource_key(&folder_id);
        let Some(warm) = context
            .business()
            .priority(
                "gui-active-folder-cache-warm",
                active_folder_cache_warm_priority(),
            )
            .cancellable()
            .exclusive_for(
                &mut self.waveform.cache.active_folder_warm_tasks,
                key.clone(),
            )
        else {
            return;
        };
        self.waveform.cache.active_folder_warm_key = Some(key);
        self.waveform.cache.active_folder_warm_cancel = Some(warm.stream(
            move |worker_context: BusinessWorkContext,
                  events: BusinessEventSink<ActiveFolderCacheWarmProgress>| {
                let progress_events = events.clone();
                warm_active_folder_waveform_cache_with_progress(
                    folder_id,
                    paths,
                    || worker_context.is_cancelled(),
                    |progress| {
                        let _ = worker_context
                            .yield_if_elapsed(ACTIVE_FOLDER_CACHE_PROGRESS_YIELD_INTERVAL);
                        let _ = progress_events.emit(progress);
                    },
                )
            },
            GuiMessage::ActiveFolderCacheWarmProgress,
            GuiMessage::ActiveFolderCacheWarmFinished,
        ));
    }

    pub(in crate::native_app) fn apply_active_folder_cache_warm_progress(
        &mut self,
        completion: ui::KeyedTaskCompletion<ui::ResourceKey, ActiveFolderCacheWarmProgress>,
    ) {
        if !self
            .waveform
            .cache
            .active_folder_warm_tasks
            .is_active_key(&completion.key, completion.ticket)
        {
            return;
        }
        let progress = completion.output;
        if self.waveform.cache.active_folder_warm_folder_id.as_deref()
            != Some(progress.folder_id.as_str())
        {
            return;
        }
        self.waveform.cache.active_folder_warm_completed = self
            .waveform
            .cache
            .active_folder_warm_batch_base_completed
            .saturating_add(progress.processed)
            .min(self.waveform.cache.active_folder_warm_total);
        let cached_file_id = progress.cached.then(|| progress.path.display().to_string());
        self.waveform.cache.active_folder_warm_current = Some(progress.path);
        self.waveform.cache.active_folder_warm_current_progress =
            progress.current_progress.clamp(0.0, 1.0);
        self.waveform.cache.active_folder_warm_current_stage = Some(progress.stage);
        if let Some(file_id) = cached_file_id {
            self.waveform.cache.cached_sample_paths.insert(file_id);
        }
    }

    pub(in crate::native_app) fn pause_active_folder_cache_warm(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let running = self
            .waveform
            .cache
            .active_folder_warm_key
            .as_ref()
            .and_then(|key| self.waveform.cache.active_folder_warm_tasks.active(key))
            .is_some();
        if let Some(token) = self.waveform.cache.active_folder_warm_cancel.take() {
            token.cancel();
        }
        if running {
            return;
        }
        if let Some(key) = self.waveform.cache.active_folder_warm_key.take() {
            self.waveform.cache.active_folder_warm_tasks.cancel(&key);
        }
        self.waveform.cache.active_folder_warm_current = None;
        self.waveform.cache.active_folder_warm_current_progress = 0.0;
        self.waveform.cache.active_folder_warm_current_stage = None;
        self.reschedule_active_folder_cache_warm_delay(
            context,
            ACTIVE_FOLDER_CACHE_WARM_CONTINUATION_DELAY,
        );
    }

    fn reschedule_active_folder_cache_warm_delay(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        delay: std::time::Duration,
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
            delay,
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
        for path in result.playback_ready {
            self.waveform
                .cache
                .cached_sample_paths
                .insert(path.display().to_string());
        }
        for path in result.deferred.iter().rev() {
            self.waveform
                .cache
                .active_folder_warm_pending
                .push_front(path.clone());
        }
        self.waveform.cache.active_folder_warm_completed = self
            .waveform
            .cache
            .active_folder_warm_batch_base_completed
            .saturating_add(result.processed)
            .min(self.waveform.cache.active_folder_warm_total);
        self.waveform.cache.active_folder_warm_current = None;
        self.waveform.cache.active_folder_warm_current_progress = 0.0;
        self.waveform.cache.active_folder_warm_current_stage = None;
        if result.cancelled {
            if self.waveform.cache.active_folder_warm_pending.is_empty() {
                self.waveform.cache.active_folder_warm_folder_id = None;
                self.waveform.cache.active_folder_warm_completed = 0;
                self.waveform.cache.active_folder_warm_total = 0;
                self.waveform.cache.active_folder_warm_batch_base_completed = 0;
            } else {
                self.reschedule_active_folder_cache_warm_delay(
                    context,
                    ACTIVE_FOLDER_CACHE_WARM_CONTINUATION_DELAY,
                );
            }
            return;
        }
        log_slow_cache_phase(
            "browser.sample_cache.active_folder_finish",
            Path::new(&result.folder_id),
            started_at,
        );
        if self.waveform.cache.active_folder_warm_pending.is_empty() {
            self.waveform.cache.active_folder_warm_folder_id = None;
            self.waveform.cache.active_folder_warm_completed = 0;
            self.waveform.cache.active_folder_warm_total = 0;
            self.waveform.cache.active_folder_warm_batch_base_completed = 0;
        } else {
            let delay = if result.decoded_source {
                ACTIVE_FOLDER_CACHE_WARM_CONTINUATION_DELAY
            } else {
                ACTIVE_FOLDER_CACHE_WARM_LIGHT_CONTINUATION_DELAY
            };
            self.reschedule_active_folder_cache_warm_delay(context, delay);
        }
    }

    fn next_active_folder_cache_warm_batch(&mut self) -> Vec<std::path::PathBuf> {
        let entries = &self.waveform.cache.entries;
        self.waveform.cache.active_folder_warm_batch_base_completed =
            self.waveform.cache.active_folder_warm_completed;
        self.waveform.cache.active_folder_warm_current_progress = 0.0;
        self.waveform.cache.active_folder_warm_current_stage = None;
        let batch = take_cache_warm_batch(
            &mut self.waveform.cache.active_folder_warm_pending,
            |path| entries.contains_key(path),
            ACTIVE_FOLDER_CACHE_WARM_SCAN_MAX_FILES,
        );
        self.waveform.cache.active_folder_warm_current = batch.first().cloned();
        batch
    }
}
