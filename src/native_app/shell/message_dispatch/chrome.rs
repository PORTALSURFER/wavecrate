use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::waveform::{SimilarSectionsResult, execute_similar_sections_scan};

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
            GuiMessage::ReleaseUpdateCheckFinished(completion) => {
                self.finish_release_update_check(completion);
            }
            GuiMessage::OpenReleaseDownloadPage => {
                self.open_release_download_page();
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
            GuiMessage::ToggleHarvestFamilyPanel => {
                self.ui.chrome.harvest_family_open = !self.ui.chrome.harvest_family_open;
            }
            GuiMessage::ToggleZeroCrossingSnap => {
                let enabled = self.waveform.current.toggle_zero_crossing_snap();
                self.ui.status.sample = if enabled {
                    String::from("Zero crossing snap enabled")
                } else {
                    String::from("Zero crossing snap disabled")
                };
            }
            GuiMessage::ToggleBeatGuides => {
                self.ui.chrome.beat_guides_enabled = !self.ui.chrome.beat_guides_enabled;
            }
            GuiMessage::AdjustBeatGuideCount(delta) => {
                self.ui.chrome.adjust_beat_guide_count(delta);
            }
            GuiMessage::ToggleSimilarSections => {
                self.toggle_similar_sections(context);
            }
            GuiMessage::SimilarSectionsResolved(result) => {
                self.finish_similar_sections(result);
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
                self.finish_folder_browser_rename(completion, context);
            }
            GuiMessage::DeleteSelectedItem => self.delete_selected_item(context),
            GuiMessage::RequestCropWaveformSelection => {
                self.request_crop_waveform_selection(context);
            }
            GuiMessage::RequestTrimWaveformSelection => {
                self.request_trim_waveform_selection(context);
            }
            GuiMessage::RequestReverseWaveformSelection => {
                self.request_reverse_waveform_selection(context);
            }
            GuiMessage::RequestExtractAndTrimWaveformSelection => {
                self.request_extract_and_trim_waveform_selection(context);
            }
            GuiMessage::RequestCropPlaymarkSelection => {
                self.request_crop_playmark_selection(context);
            }
            GuiMessage::RequestTrimPlaymarkSelection => {
                self.request_trim_playmark_selection(context);
            }
            GuiMessage::RequestReversePlaymarkSelection => {
                self.request_reverse_playmark_selection(context);
            }
            GuiMessage::RequestExtractAndTrimPlaymarkSelection => {
                self.request_extract_and_trim_playmark_selection(context);
            }
            GuiMessage::RequestApplyEditSelectionEffects => {
                self.request_apply_edit_selection_effects(context);
            }
            GuiMessage::OpenContextMenu => {
                self.open_context_menu_from_shortcut();
            }
            GuiMessage::ConfirmPendingWaveformDestructiveEdit => {
                self.confirm_pending_waveform_destructive_edit(context);
            }
            GuiMessage::CancelPendingWaveformDestructiveEdit => {
                self.cancel_pending_waveform_destructive_edit();
            }
            GuiMessage::WaveformDestructiveEditFinished(completion) => {
                self.finish_waveform_destructive_edit(completion, context);
            }
            GuiMessage::ExtractPlaymarkedRange => self.extract_playmarked_range(context),
            GuiMessage::ExtractPlaymarkedRangeToHarvestDestination => {
                self.extract_playmarked_range_to_harvest_destination(context);
            }
            GuiMessage::PlaySelectionExtractionFinished {
                completion,
                drag_position,
                playback_type,
                harvest_operation,
                started_at,
            } => self.finish_play_selection_extraction(
                completion,
                drag_position,
                playback_type,
                harvest_operation,
                started_at,
                context,
            ),
            _ => unreachable!("chrome dispatcher received a non-chrome message"),
        }
    }

    fn toggle_similar_sections(&mut self, context: &mut ui::UiUpdateContext<GuiMessage>) {
        if self.waveform.current.similar_sections_enabled() {
            self.waveform.current.clear_similar_sections();
            self.ui.status.sample = String::from("Similar section marks off");
            return;
        }

        let request = match self.waveform.current.similar_sections_request() {
            Ok(request) => request,
            Err(error) => {
                self.ui.status.sample = error;
                return;
            }
        };
        let anchor = request.anchor();
        self.waveform.current.start_similar_sections(anchor);
        self.ui.status.sample = String::from("Finding similar sections");
        context
            .business()
            .background("gui-waveform-similar-sections")
            .run(
                move |_| execute_similar_sections_scan(request),
                GuiMessage::SimilarSectionsResolved,
            );
    }

    fn finish_similar_sections(&mut self, result: SimilarSectionsResult) {
        if !self
            .waveform
            .current
            .similar_sections_result_applies(&result)
        {
            return;
        }
        match result.result {
            Ok(payload) => {
                let count = payload.ranges.len();
                self.waveform
                    .current
                    .finish_similar_sections_scan(payload.ranges);
                self.ui.status.sample = if count == 0 {
                    String::from("No similar sections found")
                } else {
                    format!(
                        "Found {count} similar section{}",
                        if count == 1 { "" } else { "s" }
                    )
                };
            }
            Err(error) => {
                self.waveform.current.clear_similar_sections();
                self.ui.status.sample = error;
            }
        }
    }
}
