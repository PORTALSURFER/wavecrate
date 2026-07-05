use radiant::{prelude as ui, widgets::PointerModifiers};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label};
use crate::native_app::starmap_audition_telemetry::{
    self as starmap_telemetry, StarmapAuditionCounter, StarmapAuditionDuration,
};

use super::validation_worker;

const SAMPLE_LOAD_VALIDATION_TASK_NAME: &str = "gui-sample-load-validate";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SampleLoadPathValidationRequest {
    pub(in crate::native_app::audio::sample_load_actions) path: String,
    pub(in crate::native_app::audio::sample_load_actions) intent: SampleLoadPathValidationIntent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SampleLoadPathValidation {
    pub(in crate::native_app) path: String,
    intent: SampleLoadPathValidationIntent,
    existing_file: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app::audio::sample_load_actions) enum SampleLoadPathValidationIntent {
    Foreground { autoplay: bool },
    Selection { autoplay: bool },
}

impl SampleLoadPathValidationRequest {
    fn new(path: String, intent: SampleLoadPathValidationIntent) -> Self {
        Self { path, intent }
    }
}

impl SampleLoadPathValidation {
    pub(super) fn existing(request: SampleLoadPathValidationRequest, existing_file: bool) -> Self {
        Self {
            path: request.path,
            intent: request.intent,
            existing_file,
        }
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn select_sample(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        self.library
            .folder_browser
            .focus_file_preserving_selection_matching_tags(
                path.clone(),
                &self.metadata.tags_by_file,
            );
        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.audio.pending_sample_playback = None;
        self.start_selection_fast_audition(path.as_str(), context, started_at);
        self.queue_sample_load_path_validation(
            path,
            SampleLoadPathValidationIntent::Selection { autoplay: true },
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn select_sample_with_modifiers(
        &mut self,
        path: String,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        self.library
            .folder_browser
            .select_file_with_modifiers_matching_tags(
                path.clone(),
                modifiers,
                &self.metadata.tags_by_file,
            );
        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.audio.pending_sample_playback = None;
        self.start_selection_fast_audition(path.as_str(), context, started_at);
        self.queue_sample_load_path_validation(
            path,
            SampleLoadPathValidationIntent::Selection { autoplay: true },
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn start_starmap_drag_audition_sample(
        &mut self,
        path: String,
        _modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let total_started_at = starmap_telemetry::stage_timer();
        let started_at = Instant::now();
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let focus_started_at = starmap_telemetry::stage_timer();
        self.library
            .folder_browser
            .select_known_starmap_file_for_audition(path.clone());
        let focus_elapsed = starmap_telemetry::elapsed_since(focus_started_at);
        if let Some(elapsed) = focus_elapsed {
            starmap_telemetry::record_duration(StarmapAuditionDuration::Focus, elapsed);
        }
        let selection_changed =
            self.library.folder_browser.selected_file_id() != previous_selection.as_deref();
        starmap_telemetry::record_event(
            None,
            "sample_start.focus",
            if selection_changed {
                "selection_changed"
            } else {
                "same_selection"
            },
            Some(path.as_str()),
            1,
            self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
            self.ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some(),
            focus_elapsed,
        );
        if selection_changed {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.audio.pending_sample_playback = None;
        if self.start_loaded_navigation_sample(path.as_str(), context, started_at) {
            starmap_telemetry::record_event(
                Some(StarmapAuditionCounter::LoadedCurrent),
                "sample_start.loaded_current",
                "started",
                Some(path.as_str()),
                1,
                self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
                self.ui
                    .chrome
                    .starmap_audition_queue
                    .active_file_id
                    .is_some(),
                starmap_telemetry::elapsed_since(total_started_at),
            );
            if let Some(elapsed) = starmap_telemetry::elapsed_since(total_started_at) {
                starmap_telemetry::record_duration(StarmapAuditionDuration::StartTotal, elapsed);
            }
            if !self
                .ui
                .chrome
                .starmap_audition_queue
                .queued_file_ids
                .is_empty()
            {
                self.advance_starmap_drag_audition_latest_immediately(context);
            }
            return;
        }
        let ready_started_at = starmap_telemetry::stage_timer();
        let ready_outcome = self.start_fast_path_audition(
            path.as_str(),
            context,
            started_at,
            super::cache_start::FastAuditionOptions::starmap_drag(),
        );
        let ready_elapsed = starmap_telemetry::elapsed_since(ready_started_at);
        if let Some(elapsed) = ready_elapsed {
            starmap_telemetry::record_duration(StarmapAuditionDuration::ReadySource, elapsed);
        }
        let ready_counter = match ready_outcome {
            super::cache_start::InstantAuditionOutcome::Started => {
                Some(StarmapAuditionCounter::ReadyStarted)
            }
            super::cache_start::InstantAuditionOutcome::AudioPending => {
                Some(StarmapAuditionCounter::ReadyPending)
            }
            super::cache_start::InstantAuditionOutcome::Unavailable => {
                Some(StarmapAuditionCounter::ReadyUnavailable)
            }
        };
        starmap_telemetry::record_event(
            ready_counter,
            "sample_start.ready_source",
            ready_outcome.as_str(),
            Some(path.as_str()),
            1,
            self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
            self.ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some(),
            ready_elapsed,
        );
        if ready_outcome.uses_ready_source() {
            if ready_outcome == super::cache_start::InstantAuditionOutcome::Started
                && !self
                    .ui
                    .chrome
                    .starmap_audition_queue
                    .queued_file_ids
                    .is_empty()
            {
                self.advance_starmap_drag_audition_latest_immediately(context);
            }
            if let Some(elapsed) = starmap_telemetry::elapsed_since(total_started_at) {
                starmap_telemetry::record_duration(StarmapAuditionDuration::StartTotal, elapsed);
            }
            return;
        }
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::ValidationQueued),
            "sample_start.validation",
            "queued",
            Some(path.as_str()),
            1,
            self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
            self.ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some(),
            starmap_telemetry::elapsed_since(total_started_at),
        );
        if let Some(elapsed) = starmap_telemetry::elapsed_since(total_started_at) {
            starmap_telemetry::record_duration(StarmapAuditionDuration::StartTotal, elapsed);
        }
        self.queue_sample_load_path_validation(
            path,
            SampleLoadPathValidationIntent::Selection { autoplay: true },
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn promote_starmap_audition_sample(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.queue_sample_load_path_validation(
            path,
            SampleLoadPathValidationIntent::Selection { autoplay: true },
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn load_sample(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.audio.pending_sample_playback = None;
        let started_at = Instant::now();
        self.queue_sample_load_path_validation(
            path,
            SampleLoadPathValidationIntent::Foreground { autoplay: true },
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn load_sample_without_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.queue_sample_load_path_validation(
            path,
            SampleLoadPathValidationIntent::Foreground { autoplay: false },
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn load_validated_sample_without_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        self.load_sample_with_autoplay_validated(path, context, false, started_at);
    }

    pub(in crate::native_app) fn load_navigation_sample(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.background.sample_load_validation_task.cancel();
        self.load_navigation_sample_validated(path, context, started_at);
    }

    fn load_sample_with_autoplay_validated(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        autoplay: bool,
        started_at: Instant,
    ) {
        self.yield_sample_cache_warm_for_foreground_load(context);
        self.cancel_inflight_sample_load_preserving_early_playback_for(path.as_str());
        if self.start_memory_cached_sample(path.as_str(), autoplay, context, started_at) {
            return;
        }
        let instant_audition_started = autoplay
            && self
                .start_fast_path_audition(
                    path.as_str(),
                    context,
                    started_at,
                    super::cache_start::FastAuditionOptions::instant_navigation(),
                )
                .uses_ready_source();
        self.start_foreground_sample_load_with_priority(
            path.as_str(),
            autoplay,
            context,
            started_at,
            if instant_audition_started {
                ui::TaskPriority::Background
            } else {
                ui::TaskPriority::Interactive
            },
            if instant_audition_started {
                "waveform_load_after_instant_audition"
            } else {
                "foreground_load_queued"
            },
            if instant_audition_started {
                crate::native_app::audio::sample_load_actions::types::SampleLoadStrategy::DisplayAfterInstantAudition
            } else {
                crate::native_app::audio::sample_load_actions::types::SampleLoadStrategy::CacheThenDecode
            },
        );
    }

    pub(in crate::native_app) fn load_navigation_sample_validated(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        self.yield_sample_cache_warm_for_foreground_load(context);
        self.cancel_inflight_sample_load_preserving_early_playback_for(path.as_str());
        self.audio.pending_sample_playback = None;
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        if self.start_loaded_navigation_sample(path.as_str(), context, started_at) {
            return;
        }
        if self.start_memory_cached_sample(path.as_str(), true, context, started_at) {
            return;
        }
        let instant_audition_started = self
            .start_fast_path_audition(
                path.as_str(),
                context,
                started_at,
                super::cache_start::FastAuditionOptions::instant_navigation(),
            )
            .uses_ready_source();
        self.start_foreground_sample_load_with_priority(
            path.as_str(),
            true,
            context,
            started_at,
            if instant_audition_started {
                ui::TaskPriority::Background
            } else {
                ui::TaskPriority::Interactive
            },
            if instant_audition_started {
                "waveform_load_after_instant_audition"
            } else {
                "foreground_load_queued"
            },
            if instant_audition_started {
                crate::native_app::audio::sample_load_actions::types::SampleLoadStrategy::DisplayAfterInstantAudition
            } else {
                crate::native_app::audio::sample_load_actions::types::SampleLoadStrategy::CacheThenDecode
            },
        );
    }

    fn queue_sample_load_path_validation(
        &mut self,
        path: String,
        intent: SampleLoadPathValidationIntent,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.yield_sample_cache_warm_for_foreground_load(context);
        self.cancel_inflight_sample_load_preserving_early_playback_for(path.as_str());
        let request = SampleLoadPathValidationRequest::new(path, intent);
        context
            .business()
            .interactive(SAMPLE_LOAD_VALIDATION_TASK_NAME)
            .latest(&mut self.background.sample_load_validation_task)
            .run(
                move |_| validation_worker::validate_sample_load_path(request),
                move |completion| GuiMessage::SampleLoadPathValidated {
                    completion,
                    started_at,
                },
            );
    }

    pub(in crate::native_app) fn finish_sample_load_path_validation(
        &mut self,
        completion: ui::TaskCompletion<SampleLoadPathValidation>,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(validation) = self
            .background
            .sample_load_validation_task
            .finish_completion(completion)
        else {
            return;
        };
        self.log_sample_identity_checkpoint(
            "browser.sample_load.validation_finished",
            if validation.existing_file {
                "existing_file"
            } else {
                "missing_file"
            },
            Some(Path::new(&validation.path)),
            None,
        );
        if !validation.existing_file
            && self.prune_missing_sample_after_validation(validation.path.as_str(), started_at)
        {
            return;
        }
        if !self.validated_sample_load_is_current_browser_selection(&validation) {
            self.audio.pending_sample_playback = None;
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(validation.path.as_str())),
                "validation_stale_selection",
                started_at,
                None,
            );
            return;
        }
        match validation.intent {
            SampleLoadPathValidationIntent::Foreground { autoplay } => self
                .load_sample_with_autoplay_validated(
                    validation.path,
                    context,
                    autoplay,
                    started_at,
                ),
            SampleLoadPathValidationIntent::Selection { autoplay } => self
                .load_sample_with_autoplay_validated(
                    validation.path,
                    context,
                    autoplay,
                    started_at,
                ),
        }
    }

    fn validated_sample_load_is_current_browser_selection(
        &self,
        validation: &SampleLoadPathValidation,
    ) -> bool {
        match validation.intent {
            SampleLoadPathValidationIntent::Selection { .. } => self
                .library
                .folder_browser
                .selected_file_id()
                .is_some_and(|selected| selected == validation.path),
            SampleLoadPathValidationIntent::Foreground { .. } => true,
        }
    }

    fn prune_missing_sample_after_validation(&mut self, path: &str, started_at: Instant) -> bool {
        let absolute_path = PathBuf::from(path);
        let Some((source, relative_path)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&absolute_path)
        else {
            return false;
        };
        let changed = self
            .library
            .folder_browser
            .refresh_filesystem_paths(source.id.as_str(), &[relative_path]);
        if !changed {
            return false;
        }

        self.audio.pending_sample_playback = None;
        if self.audio.early_sample_playback_path.as_deref() == Some(path) {
            self.stop_audio_output_playback();
            self.audio.current_playback_span = None;
            self.audio.early_sample_playback_path = None;
        }
        self.ui.status.sample = format!("Removed missing {}", sample_path_label(path));
        if let Err(error) = self.library.folder_browser.save_source_scan_cache() {
            self.ui.status.sample =
                format!("{}; source cache not saved: {error}", self.ui.status.sample);
            emit_gui_action(
                "browser.select_sample.source_cache_persist",
                Some("browser"),
                Some(&sample_path_label(path)),
                "error",
                started_at,
                Some(&error),
            );
        }
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path)),
            "missing_pruned",
            started_at,
            None,
        );
        true
    }

    fn start_selection_fast_audition(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        self.start_fast_path_audition(
            path,
            context,
            started_at,
            super::cache_start::FastAuditionOptions::instant_navigation(),
        );
    }
}
