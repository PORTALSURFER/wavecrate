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

const STARMAP_AUDITION_ADVANCE_DELAY: Duration = Duration::from_millis(90);

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
        self.ui.chrome.starmap_audition_drag = Some(StarmapAuditionDragState {
            last_hit_file_id: path.clone(),
            last_position: position,
            modifiers,
        });
        self.ui.chrome.starmap_audition_queue = Default::default();
        self.enqueue_starmap_audition_hits(path.into_iter().collect(), modifiers, context);
    }

    fn update_starmap_audition_drag(
        &mut self,
        paths: Vec<String>,
        position: ui::Point,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(drag) = self.ui.chrome.starmap_audition_drag.as_mut() else {
            return;
        };
        drag.last_position = position;
        drag.modifiers = modifiers;
        if let Some(path) = paths.last() {
            drag.last_hit_file_id = Some(path.clone());
        }
        self.enqueue_starmap_audition_hits(paths, modifiers, context);
    }

    fn finish_starmap_audition_drag(&mut self) {
        self.ui.chrome.starmap_audition_drag = None;
        self.ui.chrome.starmap_audition_queue = Default::default();
    }

    fn enqueue_starmap_audition_hits(
        &mut self,
        paths: Vec<String>,
        _modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if paths.is_empty() {
            return;
        }
        self.ui.browser_interaction.clipboard_handoff_target = ClipboardHandoffTarget::BrowserFiles;
        self.ui.browser_interaction.context_menu = None;
        let Some(path) = paths.into_iter().last() else {
            return;
        };
        let queue = &mut self.ui.chrome.starmap_audition_queue;
        if queue.active_file_id.as_ref() == Some(&path) && queue.queued_file_ids.is_empty() {
            return;
        }
        if queue.queued_file_ids.len() == 1 && queue.queued_file_ids.front() == Some(&path) {
            return;
        }
        queue.queued_file_ids.clear();
        queue.queued_file_ids.push_back(path);
        self.ui.chrome.starmap_audition_queue.modifiers = starmap_audition_modifiers();
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
        if self
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .is_some()
        {
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
            return;
        };
        self.ui.chrome.starmap_audition_queue.active_file_id = Some(path.clone());
        if let Some(drag) = self.ui.chrome.starmap_audition_drag.as_mut() {
            drag.last_hit_file_id = Some(path.clone());
        }
        self.select_sample_with_modifiers(path, starmap_audition_modifiers(), context);
    }

    fn advance_starmap_audition(
        &mut self,
        ticket: ui::TaskTicket,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.background.starmap_audition_advance_task.finish(ticket) {
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
