use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_chrome_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::ToggleJobDetails => {
                self.ui.chrome.job_details_open =
                    self.library.folder_scan_active() && !self.ui.chrome.job_details_open;
            }
            GuiMessage::CloseJobDetails => {
                self.ui.chrome.job_details_open = false;
            }
            GuiMessage::ToggleShortcutHelp => {
                self.ui.chrome.shortcut_help_open = !self.ui.chrome.shortcut_help_open;
            }
            GuiMessage::CloseShortcutHelp => {
                self.ui.chrome.shortcut_help_open = false;
            }
            GuiMessage::ToggleStickyRandomSampleRangePlayback => {
                self.ui.chrome.sticky_random_sample_range_playback =
                    !self.ui.chrome.sticky_random_sample_range_playback;
                self.ui.status.sample = if self.ui.chrome.sticky_random_sample_range_playback {
                    String::from("Sticky random playback on: Space plays random sample sections")
                } else {
                    String::from("Sticky random playback off: Space plays selected samples")
                };
            }
            GuiMessage::ToggleBeatGuides => {
                self.ui.chrome.beat_guides_enabled = !self.ui.chrome.beat_guides_enabled;
            }
            GuiMessage::AdjustBeatGuideCount(delta) => {
                self.ui.chrome.adjust_beat_guide_count(delta);
            }
            GuiMessage::UndoTransaction => self.undo_transaction(),
            GuiMessage::RedoTransaction => self.redo_transaction(),
            GuiMessage::ToggleTransactionList => self.toggle_transaction_list(),
            GuiMessage::CloseTransactionList => {
                self.ui.chrome.transaction_list_open = false;
            }
            GuiMessage::FocusRenameInput(input_id) => {
                self.focus_rename_input(input_id, context);
            }
            GuiMessage::FolderBrowserRenameFinished(completion) => {
                self.finish_folder_browser_rename(completion);
            }
            GuiMessage::DeleteSelectedItem => self.delete_selected_item(context),
            GuiMessage::RequestCropWaveformSelection => {
                self.request_crop_waveform_selection(context);
            }
            GuiMessage::RequestTrimWaveformSelection => {
                self.request_trim_waveform_selection(context);
            }
            GuiMessage::RequestExtractAndTrimWaveformSelection => {
                self.request_extract_and_trim_waveform_selection(context);
            }
            GuiMessage::RequestApplyEditSelectionEffects => {
                self.request_apply_edit_selection_effects(context);
            }
            GuiMessage::ConfirmPendingWaveformDestructiveEdit => {
                self.confirm_pending_waveform_destructive_edit(context);
            }
            GuiMessage::CancelPendingWaveformDestructiveEdit => {
                self.cancel_pending_waveform_destructive_edit();
            }
            GuiMessage::ExtractPlaymarkedRange => self.extract_playmarked_range(context),
            GuiMessage::PlaySelectionExtractionFinished {
                completion,
                drag_position,
                started_at,
            } => self.finish_play_selection_extraction(
                completion,
                drag_position,
                started_at,
                context,
            ),
            _ => unreachable!("chrome dispatcher received a non-chrome message"),
        }
    }
}
