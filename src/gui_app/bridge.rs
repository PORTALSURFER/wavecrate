//! Native runtime bridge between sempal controller state and `radiant`.

use crate::{
    app_core::controller::AppController,
    app_core::native_shell::{
        normalized_from_milli, project_app_model, selection_range_from_milli,
    },
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

    fn project_model(&mut self) -> AppModel {
        project_app_model(&mut self.controller)
    }

    fn on_select_column(&mut self, target_index: usize) {
        self.controller.select_column_by_index(target_index);
    }

    fn move_browser_focus(&mut self, delta: i8) {
        let _ = self.controller.focus_browser_delta(delta);
    }

    fn delete_browser_selection(&mut self) {
        let _ = self.controller.delete_active_browser_selection();
    }

    fn tag_browser_selection(&mut self, target: BrowserTagTarget) {
        let rating = match target {
            BrowserTagTarget::Trash => crate::sample_sources::Rating::TRASH_3,
            BrowserTagTarget::Neutral => crate::sample_sources::Rating::NEUTRAL,
            BrowserTagTarget::Keep => crate::sample_sources::Rating::KEEP_1,
        };
        self.controller.tag_selected(rating);
    }

    fn confirm_active_prompt(&mut self) {
        let _ = self.controller.confirm_active_prompt();
    }

    fn cancel_active_prompt(&mut self) {
        let _ = self.controller.cancel_active_prompt();
    }

    fn move_folder_focus(&mut self, delta: i8) {
        self.controller
            .nudge_folder_selection(delta as isize, false);
    }

    fn set_browser_tab(&mut self, map: bool) {
        self.controller.set_browser_tab(map);
    }

    fn focus_map_sample(&mut self, sample_id: String) {
        self.controller.focus_map_sample_and_preview(&sample_id);
    }

    fn set_active_prompt_input(&mut self, value: String) {
        if self.controller.set_browser_rename_input(value.clone()) {
            return;
        }
        if self.controller.set_folder_rename_input(value.clone()) {
            return;
        }
        self.controller.set_new_folder_creation_input(value);
    }
}

impl NativeAppBridge for SempalNativeBridge {
    fn pull_model(&mut self) -> AppModel {
        self.controller.tick_playhead();
        self.controller.poll_background_jobs();
        self.controller.update_performance_governor(false);
        self.project_model()
    }

    fn on_action(&mut self, action: UiAction) {
        match action {
            UiAction::SelectColumn { index } => self.on_select_column(index),
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
            UiAction::MoveFolderFocus { delta } => self.move_folder_focus(delta),
            UiAction::StartNewFolder => self.controller.start_new_folder(),
            UiAction::StartNewFolderAtRoot => self.controller.start_new_folder_at_root(),
            UiAction::StartFolderRename => self.controller.start_folder_rename(),
            UiAction::DeleteFocusedFolder => self.controller.delete_focused_folder(),
            UiAction::ClearFolderDeleteRecoveryLog => {
                self.controller.clear_folder_delete_recovery_log()
            }
            UiAction::MoveBrowserFocus { delta } => self.move_browser_focus(delta),
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
                let _ = self.controller.extend_browser_selection_delta(delta, false);
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                let _ = self.controller.extend_browser_selection_delta(delta, true);
            }
            UiAction::ToggleFocusedBrowserRowSelection => {
                self.controller.toggle_focused_selection()
            }
            UiAction::SelectAllBrowserRows => self.controller.select_all_browser_rows(),
            UiAction::SetBrowserSearch { query } => self.controller.set_browser_search(query),
            UiAction::SetBrowserTab { map } => self.set_browser_tab(map),
            UiAction::FocusMapSample { sample_id } => self.focus_map_sample(sample_id),
            UiAction::SetPromptInput { value } => self.set_active_prompt_input(value),
            UiAction::StartBrowserRename => self.controller.start_browser_rename(),
            UiAction::ConfirmBrowserRename => self.controller.apply_pending_browser_rename(),
            UiAction::CancelBrowserRename => self.controller.cancel_browser_rename(),
            UiAction::TagBrowserSelection { target } => self.tag_browser_selection(target),
            UiAction::DeleteBrowserSelection => self.delete_browser_selection(),
            UiAction::ConfirmPrompt => self.confirm_active_prompt(),
            UiAction::CancelPrompt => self.cancel_active_prompt(),
            UiAction::CancelProgress => self.controller.request_progress_cancel(),
            UiAction::ToggleLoopPlayback => self.controller.toggle_loop(),
            UiAction::SeekWaveform { position_milli } => {
                let normalized = normalized_from_milli(position_milli);
                self.controller.seek_to(normalized);
                self.controller.set_waveform_cursor(normalized);
                self.controller.focus_waveform();
            }
            UiAction::SetWaveformCursor { position_milli } => {
                self.controller
                    .set_waveform_cursor(normalized_from_milli(position_milli));
                self.controller.focus_waveform();
            }
            UiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            } => {
                self.controller
                    .set_selection_range(selection_range_from_milli(start_milli, end_milli));
                self.controller.focus_waveform();
            }
            UiAction::ClearWaveformSelection => {
                self.controller.clear_selection();
                self.controller.focus_waveform();
            }
            UiAction::ZoomWaveform { zoom_in, steps } => {
                self.controller.zoom_waveform_steps_with_factor(
                    zoom_in,
                    u32::from(steps.max(1)),
                    None,
                    None,
                    true,
                    true,
                );
                self.controller.focus_waveform();
            }
            UiAction::ZoomWaveformToSelection => {
                self.controller.zoom_waveform_to_selection();
                self.controller.focus_waveform();
            }
            UiAction::ZoomWaveformFull => {
                self.controller.zoom_waveform_full();
                self.controller.focus_waveform();
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
        if let Err(err) = self.controller.save_full_config() {
            eprintln!("Failed to persist config on native runtime exit: {err}");
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
