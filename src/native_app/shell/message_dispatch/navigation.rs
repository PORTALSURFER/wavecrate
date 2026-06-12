use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_navigation_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::NavigateBrowser { delta, extend } => {
                self.navigate_browser(delta, extend, context);
            }
            GuiMessage::ToggleSelectedSampleAndAdvance => {
                self.toggle_selected_sample_and_advance(context);
            }
            GuiMessage::SelectAllSamples => {
                self.select_all_samples();
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
            GuiMessage::ExpandSelectedFolder => {
                self.expand_selected_folder();
            }
            GuiMessage::CancelBrowserDragOnSampleList => {
                self.cancel_browser_drag_on_sample_list(context);
            }
            GuiMessage::DropWaveformSelectionOnSampleList => {
                self.drop_waveform_play_selection_on_sample_list(context);
            }
            _ => unreachable!("navigation dispatcher received a non-navigation message"),
        }
    }
}
