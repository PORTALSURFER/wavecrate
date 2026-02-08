//! Backend-neutral controller aliases for migration consumers.
//!
//! The GUI migration still uses the legacy controller implementation internally,
//! but exposing it through `app_core` gives runtimes and tooling a stable path
//! that remains valid while `app` internals are retired.

/// Transitional controller type used by native runtime bridges and migration CLIs.
pub type AppController = crate::app::controller::LegacyAppController;

use radiant::app::{AppModel, UiAction};

/// Backend-neutral status helpers for migration-facing runtime code.
pub trait AppControllerStatusExt {
    /// Set an error status message on the controller.
    ///
    /// This keeps native-bridge code independent from legacy UI style enums
    /// while migration is in progress.
    fn set_error_status(&mut self, message: impl Into<String>);
}

impl AppControllerStatusExt for AppController {
    fn set_error_status(&mut self, message: impl Into<String>) {
        AppController::set_error_status(self, message);
    }
}

/// Backend-neutral native-runtime orchestration helpers.
pub trait AppControllerNativeRuntimeExt {
    /// Apply per-frame controller maintenance before projecting the UI model.
    fn prepare_native_frame(&mut self);

    /// Project the current controller state into a native runtime app model.
    fn project_native_app_model(&mut self) -> AppModel;

    /// Persist full configuration during native runtime shutdown.
    fn persist_native_exit_config(&self) -> Result<(), String>;

    /// Apply a native runtime UI action to the controller.
    fn apply_native_ui_action(&mut self, action: UiAction);
}

impl AppControllerNativeRuntimeExt for AppController {
    fn prepare_native_frame(&mut self) {
        self.tick_playhead();
        self.poll_background_jobs();
        self.update_performance_governor(false);
    }

    fn project_native_app_model(&mut self) -> AppModel {
        crate::app_core::native_shell::project_app_model(self)
    }

    fn persist_native_exit_config(&self) -> Result<(), String> {
        self.save_full_config()
            .map_err(|err| format!("Failed to persist config on native runtime exit: {err}"))
    }

    fn apply_native_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::SelectColumn { index } => self.select_column_by_index(index),
            UiAction::MoveColumn { delta } => self.move_selection_column(delta as isize),
            UiAction::ToggleTransport => self.toggle_play_pause(),
            UiAction::FocusBrowserPanel => self.focus_browser_list(),
            UiAction::FocusSourcesPanel => self.focus_sources_list(),
            UiAction::FocusWaveformPanel => self.focus_waveform(),
            UiAction::FocusLoadedSampleInBrowser => self.focus_loaded_sample_in_browser(),
            UiAction::FocusBrowserSearch => self.focus_browser_search(),
            UiAction::FocusFolderSearch => self.focus_folder_search(),
            UiAction::SetFolderSearch { query } => self.set_folder_search(query),
            UiAction::SelectSourceRow { index } => self.select_source_by_index(index),
            UiAction::FocusFolderRow { index } => self.focus_folder_row(index),
            UiAction::MoveFolderFocus { delta } => self.nudge_folder_focus_action(delta),
            UiAction::StartNewFolder => self.start_new_folder(),
            UiAction::StartNewFolderAtRoot => self.start_new_folder_at_root(),
            UiAction::StartFolderRename => self.start_folder_rename(),
            UiAction::DeleteFocusedFolder => self.delete_focused_folder(),
            UiAction::ClearFolderDeleteRecoveryLog => self.clear_folder_delete_recovery_log(),
            UiAction::MoveBrowserFocus { delta } => self.focus_browser_delta_action(delta),
            UiAction::FocusBrowserRow { visible_row } => self.focus_browser_row(visible_row),
            UiAction::ToggleBrowserRowSelection { visible_row } => {
                self.toggle_browser_row_selection(visible_row)
            }
            UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                self.extend_browser_selection_to_row(visible_row)
            }
            UiAction::AddRangeBrowserSelection { visible_row } => {
                self.add_range_browser_selection(visible_row)
            }
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                self.extend_browser_selection_from_focus_action(delta)
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                self.add_range_browser_selection_from_focus_action(delta)
            }
            UiAction::ToggleFocusedBrowserRowSelection => self.toggle_focused_selection(),
            UiAction::SelectAllBrowserRows => self.select_all_browser_rows(),
            UiAction::SetBrowserSearch { query } => self.set_browser_search(query),
            UiAction::SetBrowserTab { map } => self.set_browser_tab(map),
            UiAction::FocusMapSample { sample_id } => self.focus_map_sample_and_preview(&sample_id),
            UiAction::SetPromptInput { value } => self.set_active_prompt_input(value),
            UiAction::StartBrowserRename => self.start_browser_rename(),
            UiAction::ConfirmBrowserRename => self.apply_pending_browser_rename(),
            UiAction::CancelBrowserRename => self.cancel_browser_rename(),
            UiAction::TagBrowserSelection { target } => {
                self.tag_selected_browser_target(target.into())
            }
            UiAction::DeleteBrowserSelection => self.delete_active_browser_selection_action(),
            UiAction::ConfirmPrompt => self.confirm_active_prompt_action(),
            UiAction::CancelPrompt => self.cancel_active_prompt_action(),
            UiAction::CancelProgress => self.request_progress_cancel(),
            UiAction::ToggleLoopPlayback => self.toggle_loop(),
            UiAction::SeekWaveform { position_milli } => self.seek_waveform_milli(position_milli),
            UiAction::SetWaveformCursor { position_milli } => {
                self.set_waveform_cursor_milli(position_milli)
            }
            UiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            } => self.set_waveform_selection_range_milli(start_milli, end_milli),
            UiAction::ClearWaveformSelection => self.clear_waveform_selection_with_focus(),
            UiAction::ZoomWaveform { zoom_in, steps } => {
                self.zoom_waveform_steps_from_ui(zoom_in, steps)
            }
            UiAction::ZoomWaveformToSelection => self.zoom_waveform_to_selection_with_focus(),
            UiAction::ZoomWaveformFull => self.zoom_waveform_full_with_focus(),
            UiAction::Undo => self.undo(),
            UiAction::Redo => self.redo(),
            UiAction::CheckForUpdates => self.check_for_updates_now(),
            UiAction::OpenUpdateLink => self.open_update_link(),
            UiAction::InstallUpdate => self.install_update_and_exit(),
            UiAction::DismissUpdate => self.dismiss_update_notification(),
        }
    }
}
