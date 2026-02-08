//! Native runtime bridge between sempal controller state and `radiant`.

use crate::{
    app_core::controller::{AppController, AppControllerNativeRuntimeExt},
    app_core::state::BrowserTagTarget as AppBrowserTagTarget,
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
use radiant::app::{AppModel, BrowserTagTarget, FrameBuildResult, NativeAppBridge, UiAction};
use std::{cell::RefCell, rc::Rc};

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: AppController,
}

impl SempalNativeBridge {
    /// Build a new native bridge initialized with persisted sempal configuration.
    pub fn new(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
    ) -> Result<Self, String> {
        let cfg = crate::sample_sources::config::load_or_default()
            .map_err(|err| format!("Failed to load config: {err}"))?;
        let mut controller = AppController::new_with_job_message_queue_capacity(
            renderer,
            player,
            cfg.core.job_message_queue_capacity as usize,
        );
        controller
            .apply_configuration(cfg)
            .map_err(|err| format!("Failed to load config: {err}"))?;
        controller.select_first_source();
        Ok(Self { controller })
    }

}

impl NativeAppBridge for SempalNativeBridge {
    fn pull_model(&mut self) -> AppModel {
        self.controller.prepare_native_frame();
        self.controller.project_native_app_model()
    }

    fn on_action(&mut self, action: UiAction) {
        match action {
            UiAction::SelectColumn { index } => self.controller.select_column_by_index(index),
            UiAction::MoveColumn { delta } => {
                self.controller.move_selection_column(delta as isize);
            }
            UiAction::ToggleTransport => self.controller.toggle_play_pause(),
            UiAction::FocusBrowserPanel => self.controller.focus_browser_list(),
            UiAction::FocusSourcesPanel => self.controller.focus_sources_list(),
            UiAction::FocusWaveformPanel => self.controller.focus_waveform(),
            UiAction::FocusLoadedSampleInBrowser => {
                self.controller.focus_loaded_sample_in_browser()
            }
            UiAction::FocusBrowserSearch => self.controller.focus_browser_search(),
            UiAction::FocusFolderSearch => self.controller.focus_folder_search(),
            UiAction::SetFolderSearch { query } => self.controller.set_folder_search(query),
            UiAction::SelectSourceRow { index } => self.controller.select_source_by_index(index),
            UiAction::FocusFolderRow { index } => self.controller.focus_folder_row(index),
            UiAction::MoveFolderFocus { delta } => self.controller.nudge_folder_focus_action(delta),
            UiAction::StartNewFolder => self.controller.start_new_folder(),
            UiAction::StartNewFolderAtRoot => self.controller.start_new_folder_at_root(),
            UiAction::StartFolderRename => self.controller.start_folder_rename(),
            UiAction::DeleteFocusedFolder => self.controller.delete_focused_folder(),
            UiAction::ClearFolderDeleteRecoveryLog => {
                self.controller.clear_folder_delete_recovery_log()
            }
            UiAction::MoveBrowserFocus { delta } => self.controller.focus_browser_delta_action(delta),
            UiAction::FocusBrowserRow { visible_row } => {
                self.controller.focus_browser_row(visible_row)
            }
            UiAction::ToggleBrowserRowSelection { visible_row } => {
                self.controller.toggle_browser_row_selection(visible_row)
            }
            UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                self.controller.extend_browser_selection_to_row(visible_row)
            }
            UiAction::AddRangeBrowserSelection { visible_row } => {
                self.controller.add_range_browser_selection(visible_row)
            }
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                self.controller.extend_browser_selection_from_focus_action(delta);
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                self.controller
                    .add_range_browser_selection_from_focus_action(delta);
            }
            UiAction::ToggleFocusedBrowserRowSelection => {
                self.controller.toggle_focused_selection()
            }
            UiAction::SelectAllBrowserRows => self.controller.select_all_browser_rows(),
            UiAction::SetBrowserSearch { query } => self.controller.set_browser_search(query),
            UiAction::SetBrowserTab { map } => self.controller.set_browser_tab(map),
            UiAction::FocusMapSample { sample_id } => {
                self.controller.focus_map_sample_and_preview(&sample_id)
            }
            UiAction::SetPromptInput { value } => self.controller.set_active_prompt_input(value),
            UiAction::StartBrowserRename => self.controller.start_browser_rename(),
            UiAction::ConfirmBrowserRename => self.controller.apply_pending_browser_rename(),
            UiAction::CancelBrowserRename => self.controller.cancel_browser_rename(),
            UiAction::TagBrowserSelection { target } => {
                let target = match target {
                    BrowserTagTarget::Trash => AppBrowserTagTarget::Trash,
                    BrowserTagTarget::Neutral => AppBrowserTagTarget::Neutral,
                    BrowserTagTarget::Keep => AppBrowserTagTarget::Keep,
                };
                self.controller.tag_selected_browser_target(target);
            }
            UiAction::DeleteBrowserSelection => {
                self.controller.delete_active_browser_selection_action()
            }
            UiAction::ConfirmPrompt => self.controller.confirm_active_prompt_action(),
            UiAction::CancelPrompt => self.controller.cancel_active_prompt_action(),
            UiAction::CancelProgress => self.controller.request_progress_cancel(),
            UiAction::ToggleLoopPlayback => self.controller.toggle_loop(),
            UiAction::SeekWaveform { position_milli } => {
                self.controller.seek_waveform_milli(position_milli);
            }
            UiAction::SetWaveformCursor { position_milli } => {
                self.controller.set_waveform_cursor_milli(position_milli);
            }
            UiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            } => {
                self.controller
                    .set_waveform_selection_range_milli(start_milli, end_milli);
            }
            UiAction::ClearWaveformSelection => {
                self.controller.clear_waveform_selection_with_focus();
            }
            UiAction::ZoomWaveform { zoom_in, steps } => {
                self.controller.zoom_waveform_steps_from_ui(zoom_in, steps);
            }
            UiAction::ZoomWaveformToSelection => {
                self.controller.zoom_waveform_to_selection_with_focus();
            }
            UiAction::ZoomWaveformFull => {
                self.controller.zoom_waveform_full_with_focus();
            }
            UiAction::Undo => self.controller.undo(),
            UiAction::Redo => self.controller.redo(),
            UiAction::CheckForUpdates => self.controller.check_for_updates_now(),
            UiAction::OpenUpdateLink => self.controller.open_update_link(),
            UiAction::InstallUpdate => self.controller.install_update_and_exit(),
            UiAction::DismissUpdate => self.controller.dismiss_update_notification(),
        }
    }

    fn on_frame_result(&mut self, _result: FrameBuildResult) {}

    fn on_exit(&mut self) {
        if let Err(err) = self.controller.persist_native_exit_config() {
            eprintln!("{err}");
        }
    }
}

/// Construct a native runtime bridge for the current sempal controller stack.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<SempalNativeBridge, String> {
    SempalNativeBridge::new(renderer, player)
}
