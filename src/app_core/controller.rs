//! Backend-neutral controller aliases for migration consumers.
//!
//! These aliases keep native runtime entrypoints stable while the runtime-agnostic
//! controller API remains sourced from the legacy `app` implementation during
//! migration.

use crate::app_core::app_api::controller::AppController as LegacyAppController;

/// Runtime-facing app controller type used by migration hosts.
pub type AppController = LegacyAppController;

use std::{cell::RefCell, rc::Rc};

use crate::app_core::actions::{NativeAppModel, NativeUiAction};
use crate::{audio::AudioPlayer, waveform::WaveformRenderer};
use tracing::{error, info};

/// Build a configured migration-facing controller for native runtime hosts.
///
/// This centralizes controller creation and config loading so native hosts need not
/// depend directly on legacy initialization details.
pub fn build_native_app_controller(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<AppController, String> {
    info!("Loading startup configuration for native app controller");
    let cfg = crate::sample_sources::config::load_or_default().map_err(|err| {
        let message = format!("Failed to load config: {err}");
        error!(err = %err, "Failed to load config for native app controller");
        message
    })?;
    info!("Startup config loaded");
    let mut controller = AppController::new_with_job_message_queue_capacity(
        renderer,
        player,
        cfg.core.job_message_queue_capacity as usize,
    );
    info!("AppController created, applying startup configuration");
    controller.apply_configuration(cfg).map_err(|err| {
        let message = format!("Failed to load config: {err}");
        error!(err = %err, "Failed to apply startup configuration");
        message
    })?;
    info!("Startup configuration applied");
    controller.select_first_source();
    info!("Selected initial source during startup");
    Ok(controller)
}

/// Backend-neutral native-runtime orchestration helpers.
pub trait AppControllerNativeRuntimeExt {
    /// Apply per-frame controller maintenance before projecting the UI model.
    ///
    /// `animation_only` enables a minimal update path for motion-only frames, for
    /// example when only waveform cursor or playhead values changed.
    fn prepare_native_frame(&mut self, animation_only: bool);

    /// Project the current controller state into a native runtime app model.
    fn project_native_app_model(&mut self) -> NativeAppModel;

    /// Project motion-only fields for incremental animation updates.
    fn project_native_motion_model(&mut self) -> crate::app_core::actions::NativeMotionModel;

    /// Persist full configuration during native runtime shutdown.
    fn persist_native_exit_config(&self) -> Result<(), String>;

    /// Apply a native runtime UI action to the controller.
    fn apply_native_ui_action(&mut self, action: NativeUiAction);
}

impl AppControllerNativeRuntimeExt for AppController {
    fn prepare_native_frame(&mut self, animation_only: bool) {
        self.flush_pending_volume_setting();
        self.flush_pending_age_update_commit();
        self.flush_pending_focused_similarity_highlight_refresh();
        self.flush_pending_waveform_seek_commit();
        self.flush_pending_waveform_image_refresh();
        if animation_only {
            self.record_frame_timing_for_fps();
            if !self.is_playing() {
                return;
            }
        }
        self.tick_playhead();
        if animation_only {
            return;
        }
        self.update_performance_governor(false);
    }

    fn project_native_app_model(&mut self) -> NativeAppModel {
        crate::app_core::native_shell::project_app_model(self)
    }

    fn project_native_motion_model(&mut self) -> crate::app_core::actions::NativeMotionModel {
        crate::app_core::native_shell::project_motion_model(self)
    }

    fn persist_native_exit_config(&self) -> Result<(), String> {
        self.save_full_config()
            .map_err(|err| format!("Failed to persist config on native runtime exit: {err}"))
    }

    fn apply_native_ui_action(&mut self, action: NativeUiAction) {
        self.begin_waveform_refresh_batch();
        match action {
            NativeUiAction::SelectColumn { index } => self.select_column_by_index(index),
            NativeUiAction::MoveColumn { delta } => self.move_selection_column(delta as isize),
            NativeUiAction::ToggleTransport => self.toggle_play_pause(),
            NativeUiAction::HandleEscape => self.handle_escape(),
            NativeUiAction::FocusBrowserPanel => self.focus_browser_list(),
            NativeUiAction::FocusSourcesPanel => self.focus_sources_list(),
            NativeUiAction::FocusWaveformPanel => self.focus_waveform(),
            NativeUiAction::FocusLoadedSampleInBrowser => self.focus_loaded_sample_in_browser(),
            NativeUiAction::FocusBrowserSearch => self.focus_browser_search(),
            NativeUiAction::FocusFolderSearch => self.focus_folder_search(),
            NativeUiAction::SetFolderSearch { query } => self.set_folder_search(query),
            NativeUiAction::SelectSourceRow { index } => self.select_source_by_index(index),
            NativeUiAction::FocusFolderRow { index } => self.focus_folder_row(index),
            NativeUiAction::MoveFolderFocus { delta } => self.nudge_folder_focus_action(delta),
            NativeUiAction::StartNewFolder => self.start_new_folder(),
            NativeUiAction::StartNewFolderAtRoot => self.start_new_folder_at_root(),
            NativeUiAction::StartFolderRename => self.start_folder_rename(),
            NativeUiAction::DeleteFocusedFolder => self.delete_focused_folder(),
            NativeUiAction::ClearFolderDeleteRecoveryLog => self.clear_folder_delete_recovery_log(),
            NativeUiAction::MoveBrowserFocus { delta } => self.focus_browser_delta_action(delta),
            NativeUiAction::FocusBrowserRow { visible_row } => {
                self.focus_browser_row_only(visible_row)
            }
            NativeUiAction::CommitFocusedBrowserRow => {
                self.commit_browser_focus_or_toggle_transport()
            }
            NativeUiAction::ToggleBrowserRowSelection { visible_row } => {
                self.toggle_browser_row_selection(visible_row)
            }
            NativeUiAction::ExtendBrowserSelectionToRow { visible_row } => {
                self.extend_browser_selection_to_row(visible_row)
            }
            NativeUiAction::AddRangeBrowserSelection { visible_row } => {
                self.add_range_browser_selection(visible_row)
            }
            NativeUiAction::ExtendBrowserSelectionFromFocus { delta } => {
                self.extend_browser_selection_from_focus_action(delta)
            }
            NativeUiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                self.add_range_browser_selection_from_focus_action(delta)
            }
            NativeUiAction::ToggleFocusedBrowserRowSelection => self.toggle_focused_selection(),
            NativeUiAction::SelectAllBrowserRows => self.select_all_browser_rows(),
            NativeUiAction::SetBrowserSearch { query } => self.set_browser_search(query),
            NativeUiAction::SetBrowserTab { map } => self.set_browser_tab(map),
            NativeUiAction::FocusMapSample { sample_id } => {
                self.focus_map_sample_and_preview(&sample_id)
            }
            NativeUiAction::SetPromptInput { value } => self.set_active_prompt_input(value),
            NativeUiAction::StartBrowserRename => self.start_browser_rename(),
            NativeUiAction::ConfirmBrowserRename => self.apply_pending_browser_rename(),
            NativeUiAction::CancelBrowserRename => self.cancel_browser_rename(),
            NativeUiAction::TagBrowserSelection { target } => {
                self.tag_selected_browser_target(target.into())
            }
            NativeUiAction::DeleteBrowserSelection => self.delete_active_browser_selection_action(),
            NativeUiAction::ConfirmPrompt => self.confirm_active_prompt_action(),
            NativeUiAction::CancelPrompt => self.cancel_active_prompt_action(),
            NativeUiAction::CancelProgress => self.request_progress_cancel(),
            NativeUiAction::ToggleLoopPlayback => self.toggle_loop(),
            NativeUiAction::SetVolume { value_milli } => {
                self.set_volume_live((f32::from(value_milli.min(1000)) / 1000.0).clamp(0.0, 1.0))
            }
            NativeUiAction::CommitVolumeSetting => self.commit_volume_setting(),
            NativeUiAction::SeekWaveform { position_milli } => {
                self.queue_waveform_seek_milli(position_milli)
            }
            NativeUiAction::SetWaveformCursor { position_milli } => {
                self.set_waveform_cursor_milli(position_milli)
            }
            NativeUiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            } => self.set_waveform_selection_range_milli(start_milli, end_milli),
            NativeUiAction::ClearWaveformSelection => self.clear_waveform_selection_with_focus(),
            NativeUiAction::ZoomWaveform { zoom_in, steps } => {
                self.zoom_waveform_steps_from_ui(zoom_in, steps)
            }
            NativeUiAction::ZoomWaveformToSelection => self.zoom_waveform_to_selection_with_focus(),
            NativeUiAction::ZoomWaveformFull => self.zoom_waveform_full_with_focus(),
            NativeUiAction::Undo => self.undo(),
            NativeUiAction::Redo => self.redo(),
            NativeUiAction::CheckForUpdates => self.check_for_updates_now(),
            NativeUiAction::OpenUpdateLink => self.open_update_link(),
            NativeUiAction::InstallUpdate => self.install_update_and_exit(),
            NativeUiAction::DismissUpdate => self.dismiss_update_notification(),
        }
        self.end_waveform_refresh_batch();
    }
}

#[cfg(test)]
mod tests {
    use super::{AppController, AppControllerNativeRuntimeExt, WaveformRenderer};
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn prepare_native_frame_animation_only_updates_fps_when_not_playing() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

        assert!(controller.average_fps().is_none());
        controller.prepare_native_frame(true);
        assert!(controller.average_fps().is_none());

        sleep(Duration::from_millis(2));
        controller.prepare_native_frame(true);

        assert!(controller.average_fps().is_some());
    }

    #[test]
    /// Native seek actions should queue deferred playback commit work.
    fn apply_native_seek_queues_deferred_seek_commit() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

        controller.apply_native_ui_action(crate::app_core::actions::NativeUiAction::SeekWaveform {
            position_milli: 420,
        });

        assert_eq!(controller.pending_waveform_seek_milli_for_test(), Some(420));
    }
}
