use radiant::prelude as ui;

use crate::native_app::app::{ClipboardHandoffTarget, GuiMessage, NativeAppState};

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
}
