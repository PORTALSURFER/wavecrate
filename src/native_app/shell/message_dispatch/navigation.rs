use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;

use crate::native_app::app::{
    ClipboardHandoffTarget, GuiMessage, NativeAppState, SampleBrowserDisplayMode,
    SampleMapAuditionDragState, SampleMapViewportChange,
};
use crate::native_app::sample_library::folder_browser::sample_map::SampleMapProjection;

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
                self.toggle_selected_sample_and_advance(context);
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
                if self.ui.chrome.sample_browser_display == SampleBrowserDisplayMode::Map {
                    self.focus_selected_sample_map_node();
                }
            }
            GuiMessage::FocusSelectedSampleMapNode => {
                self.focus_selected_sample_map_node();
            }
            GuiMessage::ChangeSampleMapViewport(change) => {
                self.ui.chrome.sample_map_viewport.apply_change(change);
            }
            GuiMessage::BeginSampleMapAuditionDrag {
                path,
                position,
                modifiers,
            } => {
                self.begin_sample_map_audition_drag(path, position, modifiers, context);
            }
            GuiMessage::UpdateSampleMapAuditionDrag {
                paths,
                position,
                modifiers,
            } => {
                self.update_sample_map_audition_drag(paths, position, modifiers, context);
            }
            GuiMessage::FinishSampleMapAuditionDrag => {
                self.ui.chrome.sample_map_audition_drag = None;
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

    fn begin_sample_map_audition_drag(
        &mut self,
        path: Option<String>,
        position: ui::Point,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.ui.chrome.sample_map_audition_drag = Some(SampleMapAuditionDragState {
            last_hit_file_id: path.clone(),
            last_position: position,
            modifiers,
        });
        self.select_sample_map_audition_hits(path.into_iter().collect(), modifiers, context);
    }

    fn update_sample_map_audition_drag(
        &mut self,
        paths: Vec<String>,
        position: ui::Point,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if let Some(drag) = self.ui.chrome.sample_map_audition_drag.as_mut() {
            drag.last_position = position;
            drag.modifiers = modifiers;
            if let Some(path) = paths.last() {
                drag.last_hit_file_id = Some(path.clone());
            }
        }
        self.select_sample_map_audition_hits(paths, modifiers, context);
    }

    fn select_sample_map_audition_hits(
        &mut self,
        paths: Vec<String>,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if paths.is_empty() {
            return;
        }
        self.ui.browser_interaction.clipboard_handoff_target = ClipboardHandoffTarget::BrowserFiles;
        self.ui.browser_interaction.context_menu = None;
        for path in paths {
            self.select_sample_with_modifiers(path, modifiers, context);
        }
    }

    fn focus_selected_sample_map_node(&mut self) {
        self.library
            .folder_browser
            .prepare_sample_map_layout(&self.metadata.tags_by_file);
        let Some((x, y)) =
            self.library
                .folder_browser
                .selected_sample_map_position(SampleMapProjection {
                    tags_by_file: &self.metadata.tags_by_file,
                })
        else {
            return;
        };
        self.ui
            .chrome
            .sample_map_viewport
            .apply_change(SampleMapViewportChange::Center { x, y });
    }
}
