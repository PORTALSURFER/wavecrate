use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use std::time::Duration;

use crate::native_app::app::{
    ClipboardHandoffTarget, GuiMessage, NativeAppState, SampleBrowserDisplayMode,
    StarmapAuditionDragState, StarmapViewportChange,
};
use crate::native_app::sample_library::folder_browser::starmap::StarmapProjection;
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS,
    SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
    SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
};
use crate::native_app::starmap_audition_telemetry::{
    self as starmap_telemetry, StarmapAuditionCounter,
};

const STARMAP_AUDITION_ADVANCE_DELAY: Duration = Duration::ZERO;

impl NativeAppState {
    pub(super) fn apply_navigation_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::NavigateBrowser {
                delta,
                extend,
                preserve_selection,
            } => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.navigate_browser(delta, extend, preserve_selection, context);
            }
            GuiMessage::ToggleSelectedSampleAndAdvance => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.toggle_focused_browser_selection(context);
            }
            GuiMessage::SelectAllSamples => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.select_all_samples();
            }
            GuiMessage::ToggleRandomNavigationMode => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.toggle_random_navigation_mode();
            }
            GuiMessage::ToggleSampleBrowserMapView => {
                self.ui.chrome.sample_browser_display = match self.ui.chrome.sample_browser_display
                {
                    SampleBrowserDisplayMode::List => SampleBrowserDisplayMode::Map,
                    SampleBrowserDisplayMode::Map => SampleBrowserDisplayMode::List,
                };
                match self.ui.chrome.sample_browser_display {
                    SampleBrowserDisplayMode::Map => self.focus_selected_starmap_node(),
                    SampleBrowserDisplayMode::List => self.focus_selected_sample_list_row(context),
                }
            }
            GuiMessage::FocusSelectedStarmapNode => {
                self.focus_selected_starmap_node();
            }
            GuiMessage::ChangeStarmapViewport(change) => {
                self.ui.chrome.starmap_viewport.apply_change(change);
            }
            GuiMessage::BeginStarmapAuditionDrag {
                path,
                position,
                modifiers,
            } => {
                self.begin_starmap_audition_drag(path, position, modifiers, context);
            }
            GuiMessage::UpdateStarmapAuditionDrag {
                paths,
                position,
                modifiers,
            } => {
                self.update_starmap_audition_drag(paths, position, modifiers, context);
            }
            GuiMessage::AdvanceStarmapAudition { ticket } => {
                self.advance_starmap_audition(ticket, context);
            }
            GuiMessage::FinishStarmapAuditionDrag => {
                self.finish_starmap_audition_drag();
            }
            GuiMessage::SampleBrowserWindowChanged(change) => {
                self.library
                    .folder_browser
                    .apply_file_view_window_change(change);
            }
            GuiMessage::FolderTreeWindowChanged(change) => {
                self.library
                    .folder_browser
                    .apply_tree_view_window_change(change);
            }
            GuiMessage::CollapseSelectedFolder => {
                self.collapse_selected_folder();
            }
            GuiMessage::CancelBrowserDragOnSampleList => {
                self.cancel_browser_drag_on_sample_list(context);
            }
            GuiMessage::DropWaveformSelectionOnSampleList => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.drop_waveform_play_selection_on_sample_list(context);
            }
            _ => unreachable!("navigation dispatcher received a non-navigation message"),
        }
    }

    fn begin_starmap_audition_drag(
        &mut self,
        path: Option<String>,
        position: ui::Point,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = starmap_telemetry::stage_timer();
        let hit_count = usize::from(path.is_some());
        self.ui.chrome.starmap_audition_drag = Some(StarmapAuditionDragState {
            last_hit_file_id: path.clone(),
            last_position: position,
            modifiers,
        });
        self.background.starmap_audition_advance_task.cancel();
        self.ui.chrome.starmap_audition_queue = Default::default();
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::DragBegin),
            "controller.drag_begin",
            if hit_count == 0 { "empty" } else { "hit" },
            path.as_deref(),
            hit_count,
            0,
            false,
            starmap_telemetry::elapsed_since(started_at),
        );
        self.enqueue_starmap_audition_hits(path.into_iter().collect(), modifiers, context);
    }

    fn update_starmap_audition_drag(
        &mut self,
        paths: Vec<String>,
        position: ui::Point,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = starmap_telemetry::stage_timer();
        let Some(drag) = self.ui.chrome.starmap_audition_drag.as_mut() else {
            starmap_telemetry::record_event(
                Some(StarmapAuditionCounter::DragUpdate),
                "controller.drag_update",
                "ignored_inactive",
                paths.last().map(String::as_str),
                paths.len(),
                self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
                false,
                starmap_telemetry::elapsed_since(started_at),
            );
            return;
        };
        drag.last_position = position;
        drag.modifiers = modifiers;
        if let Some(path) = paths.last() {
            drag.last_hit_file_id = Some(path.clone());
        }
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::DragUpdate),
            "controller.drag_update",
            if paths.is_empty() { "empty" } else { "hit" },
            paths.last().map(String::as_str),
            paths.len(),
            self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
            self.ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some(),
            starmap_telemetry::elapsed_since(started_at),
        );
        self.enqueue_starmap_audition_hits(paths, modifiers, context);
    }

    fn finish_starmap_audition_drag(&mut self) {
        let started_at = starmap_telemetry::stage_timer();
        self.background.starmap_audition_advance_task.cancel();
        self.background.preview_audition_task.cancel();
        self.background.sample_load_validation_task.cancel();
        self.background.deferred_sample_load_task.cancel();
        if let Some(token) = self.background.sample_load_cancel.take() {
            token.cancel();
        }
        if let Some(key) = self.background.active_sample_load_key.take() {
            self.background.sample_load_tasks.cancel(&key);
        }
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        self.waveform.load.selection.cancel();
        self.audio.pending_sample_playback = None;
        self.ui.chrome.starmap_audition_drag = None;
        self.ui.chrome.starmap_audition_queue = Default::default();
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::DragFinish),
            "controller.drag_finish",
            "cleared",
            None,
            0,
            0,
            false,
            starmap_telemetry::elapsed_since(started_at),
        );
    }

    fn enqueue_starmap_audition_hits(
        &mut self,
        paths: Vec<String>,
        _modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = starmap_telemetry::stage_timer();
        let hit_count = paths.len();
        if paths.is_empty() {
            starmap_telemetry::record_event(
                None,
                "controller.enqueue",
                "empty",
                None,
                0,
                self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
                self.ui
                    .chrome
                    .starmap_audition_queue
                    .active_file_id
                    .is_some(),
                starmap_telemetry::elapsed_since(started_at),
            );
            return;
        }
        self.ui.browser_interaction.clipboard_handoff_target = ClipboardHandoffTarget::BrowserFiles;
        self.ui.browser_interaction.context_menu = None;
        let drag_active = self.ui.chrome.starmap_audition_drag.is_some();
        if drag_active {
            self.background.starmap_audition_advance_task.cancel();
        }
        let mut admitted_paths = Vec::new();
        let queue = &mut self.ui.chrome.starmap_audition_queue;
        for path in paths {
            if admitted_paths.last() == Some(&path)
                || queue.queued_file_ids.back() == Some(&path)
                || (queue.queued_file_ids.is_empty()
                    && queue.active_file_id.as_ref() == Some(&path))
            {
                starmap_telemetry::record_event(
                    Some(StarmapAuditionCounter::QueueCoalesced),
                    "controller.enqueue",
                    "coalesced_duplicate",
                    Some(path.as_str()),
                    hit_count,
                    queue.queued_file_ids.len(),
                    queue.active_file_id.is_some(),
                    starmap_telemetry::elapsed_since(started_at),
                );
                continue;
            }
            admitted_paths.push(path);
        }
        if admitted_paths.is_empty() {
            starmap_telemetry::record_event(
                Some(StarmapAuditionCounter::DuplicateActive),
                "controller.enqueue",
                "duplicate_or_coalesced",
                queue.active_file_id.as_deref(),
                hit_count,
                queue.queued_file_ids.len(),
                queue.active_file_id.is_some(),
                starmap_telemetry::elapsed_since(started_at),
            );
            return;
        }
        let latest_path = admitted_paths.last().cloned();
        if drag_active {
            if queue.active_file_id.is_some() {
                starmap_telemetry::record_event(
                    Some(StarmapAuditionCounter::ActiveReplaced),
                    "controller.enqueue",
                    "replace_active_drag",
                    latest_path.as_deref(),
                    hit_count,
                    queue.queued_file_ids.len(),
                    true,
                    starmap_telemetry::elapsed_since(started_at),
                );
            }
            queue.active_file_id = None;
            queue.queued_file_ids.clear();
        }
        if drag_active && admitted_paths.len() > 1 {
            starmap_telemetry::record_event(
                Some(StarmapAuditionCounter::QueueCoalesced),
                "controller.enqueue",
                "realtime_latest",
                latest_path.as_deref(),
                hit_count,
                admitted_paths.len().saturating_sub(1),
                queue.active_file_id.is_some(),
                starmap_telemetry::elapsed_since(started_at),
            );
        }
        if drag_active {
            if let Some(path) = latest_path.clone() {
                queue.queued_file_ids.push_back(path);
            }
        } else {
            for path in admitted_paths {
                queue.queued_file_ids.push_back(path);
            }
        }
        queue.modifiers = starmap_audition_modifiers();
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::QueueAdmitted),
            "controller.enqueue",
            "admitted",
            latest_path.as_deref(),
            hit_count,
            queue.queued_file_ids.len(),
            queue.active_file_id.is_some(),
            starmap_telemetry::elapsed_since(started_at),
        );
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::HitQueued),
            "controller.enqueue",
            "queued_latest",
            queue.queued_file_ids.back().map(String::as_str),
            hit_count,
            queue.queued_file_ids.len(),
            queue.active_file_id.is_some(),
            starmap_telemetry::elapsed_since(started_at),
        );
        self.start_next_starmap_audition_hit(context);
    }

    pub(in crate::native_app) fn schedule_next_starmap_audition_hit(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self
            .ui
            .chrome
            .starmap_audition_queue
            .queued_file_ids
            .is_empty()
        {
            self.ui.chrome.starmap_audition_queue.active_file_id = None;
            self.finish_starmap_audition_queue_if_idle();
            return;
        }
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::AdvanceScheduled),
            "controller.advance_schedule",
            "scheduled",
            self.ui
                .chrome
                .starmap_audition_queue
                .queued_file_ids
                .front()
                .map(String::as_str),
            0,
            self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
            self.ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some(),
            None,
        );
        context.after_latest(
            &mut self.background.starmap_audition_advance_task,
            STARMAP_AUDITION_ADVANCE_DELAY,
            |ticket| GuiMessage::AdvanceStarmapAudition { ticket },
        );
    }

    pub(in crate::native_app) fn start_next_starmap_audition_hit(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = starmap_telemetry::stage_timer();
        if self
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .is_some()
        {
            starmap_telemetry::record_event(
                None,
                "controller.start_next",
                "blocked_active",
                self.ui
                    .chrome
                    .starmap_audition_queue
                    .active_file_id
                    .as_deref(),
                0,
                self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
                true,
                starmap_telemetry::elapsed_since(started_at),
            );
            return;
        }
        let Some(path) = self
            .ui
            .chrome
            .starmap_audition_queue
            .queued_file_ids
            .pop_front()
        else {
            self.finish_starmap_audition_queue_if_idle();
            starmap_telemetry::record_event(
                None,
                "controller.start_next",
                "idle",
                None,
                0,
                0,
                false,
                starmap_telemetry::elapsed_since(started_at),
            );
            return;
        };
        self.ui.chrome.starmap_audition_queue.active_file_id = Some(path.clone());
        if let Some(drag) = self.ui.chrome.starmap_audition_drag.as_mut() {
            drag.last_hit_file_id = Some(path.clone());
        }
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::HitStarted),
            "controller.start_next",
            "started",
            Some(path.as_str()),
            1,
            self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
            true,
            starmap_telemetry::elapsed_since(started_at),
        );
        self.start_starmap_drag_audition_sample(path, starmap_audition_modifiers(), context);
    }

    fn advance_starmap_audition(
        &mut self,
        ticket: ui::TaskTicket,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.background.starmap_audition_advance_task.finish(ticket) {
            starmap_telemetry::record_event(
                Some(StarmapAuditionCounter::AdvanceStale),
                "controller.advance",
                "stale_ticket",
                None,
                0,
                self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
                self.ui
                    .chrome
                    .starmap_audition_queue
                    .active_file_id
                    .is_some(),
                None,
            );
            return;
        }
        self.ui.chrome.starmap_audition_queue.active_file_id = None;
        self.start_next_starmap_audition_hit(context);
    }

    fn finish_starmap_audition_queue_if_idle(&mut self) {
        if self.ui.chrome.starmap_audition_drag.is_some()
            || self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some()
            || !self
                .ui
                .chrome
                .starmap_audition_queue
                .queued_file_ids
                .is_empty()
        {
            return;
        }
        self.ui.chrome.starmap_audition_queue = Default::default();
    }

    fn focus_selected_starmap_node(&mut self) {
        self.library
            .folder_browser
            .prepare_starmap_layout(&self.metadata.tags_by_file);
        let Some((x, y)) =
            self.library
                .folder_browser
                .selected_starmap_position(StarmapProjection {
                    tags_by_file: &self.metadata.tags_by_file,
                    instant_audition_sample_paths: &self
                        .waveform
                        .cache
                        .instant_audition_sample_paths,
                })
        else {
            return;
        };
        self.ui
            .chrome
            .starmap_viewport
            .apply_change(StarmapViewportChange::Center { x, y });
    }

    fn focus_selected_sample_list_row(&mut self, context: &mut ui::UiUpdateContext<GuiMessage>) {
        self.library
            .folder_browser
            .follow_selected_file_view_matching_tags(
                SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
                SAMPLE_BROWSER_OVERSCAN_ROWS,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                &self.metadata.tags_by_file,
            );
        let Some(index) = self
            .library
            .folder_browser
            .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
        else {
            return;
        };
        context.scroll_fixed_row_into_view(
            SAMPLE_BROWSER_LIST_ID,
            index,
            SAMPLE_BROWSER_ROW_HEIGHT,
            SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
            SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
            0,
        );
    }
}

fn starmap_audition_modifiers() -> PointerModifiers {
    PointerModifiers::default()
}
