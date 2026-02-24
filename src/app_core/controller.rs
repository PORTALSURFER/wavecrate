//! Backend-neutral controller aliases for migration consumers.
//!
//! These aliases keep native runtime entrypoints stable while the runtime-agnostic
//! controller API remains sourced from the legacy `app` implementation during
//! migration.

use crate::app_core::app_api::controller::AppController as LegacyAppController;

/// Runtime-facing app controller type used by migration hosts.
pub type AppController = LegacyAppController;
/// Retained browser-row cache entry used by native-shell projection helpers.
pub(crate) type ProjectedBrowserRowCacheEntry =
    crate::app_core::app_api::controller::ProjectedBrowserRowCacheEntry;
/// Retained map-point cache key used by native-shell projection helpers.
pub(crate) type ProjectedMapPointsCacheKey =
    crate::app_core::app_api::controller::ProjectedMapPointsCacheKey;
/// Retained normalized map-point cache entry used by native-shell projection helpers.
pub(crate) type ProjectedMapPointCacheEntry =
    crate::app_core::app_api::controller::ProjectedMapPointCacheEntry;

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
                let _ = self.refresh_projection_revision_bus();
                return;
            }
        }
        self.tick_playhead();
        let _ = self.refresh_projection_revision_bus();
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
        let action = match apply_transport_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return;
            }
            Err(action) => action,
        };
        let action = match apply_browser_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return;
            }
            Err(action) => action,
        };
        let action = match apply_map_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return;
            }
            Err(action) => action,
        };
        let action = match apply_waveform_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return;
            }
            Err(action) => action,
        };
        if let Err(unhandled) = apply_prompt_and_update_native_ui_action(self, action) {
            debug_assert!(
                false,
                "native ui action was not handled by any dispatcher group: {unhandled:?}"
            );
        }
        self.end_waveform_refresh_batch();
    }
}

/// Try to dispatch transport-oriented native actions.
fn apply_transport_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SelectColumn { index } => controller.select_column_by_index(index),
        NativeUiAction::MoveColumn { delta } => controller.move_selection_column(delta as isize),
        NativeUiAction::ToggleTransport => controller.toggle_play_pause(),
        NativeUiAction::HandleEscape => controller.handle_escape(),
        NativeUiAction::ToggleLoopPlayback => controller.toggle_loop(),
        NativeUiAction::SetVolume { value_milli } => {
            controller.set_volume_live((f32::from(value_milli.min(1000)) / 1000.0).clamp(0.0, 1.0))
        }
        NativeUiAction::CommitVolumeSetting => controller.commit_volume_setting(),
        NativeUiAction::Undo => controller.undo(),
        NativeUiAction::Redo => controller.redo(),
        action => return Err(action),
    }
    Ok(())
}

/// Try to dispatch browser-and-sources native actions.
fn apply_browser_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::FocusBrowserPanel => controller.focus_browser_list(),
        NativeUiAction::FocusSourcesPanel => controller.focus_sources_list(),
        NativeUiAction::FocusWaveformPanel => controller.focus_waveform(),
        NativeUiAction::FocusLoadedSampleInBrowser => controller.focus_loaded_sample_in_browser(),
        NativeUiAction::FocusBrowserSearch => controller.focus_browser_search(),
        NativeUiAction::FocusFolderSearch => controller.focus_folder_search(),
        NativeUiAction::SetFolderSearch { query } => controller.set_folder_search(query),
        NativeUiAction::SelectSourceRow { index } => controller.select_source_by_index(index),
        NativeUiAction::FocusFolderRow { index } => controller.focus_folder_row(index),
        NativeUiAction::MoveFolderFocus { delta } => controller.nudge_folder_focus_action(delta),
        NativeUiAction::StartNewFolder => controller.start_new_folder(),
        NativeUiAction::StartNewFolderAtRoot => controller.start_new_folder_at_root(),
        NativeUiAction::StartFolderRename => controller.start_folder_rename(),
        NativeUiAction::DeleteFocusedFolder => controller.delete_focused_folder(),
        NativeUiAction::ClearFolderDeleteRecoveryLog => {
            controller.clear_folder_delete_recovery_log()
        }
        NativeUiAction::MoveBrowserFocus { delta } => controller.focus_browser_delta_action(delta),
        NativeUiAction::FocusBrowserRow { visible_row } => {
            controller.focus_browser_row_only(visible_row)
        }
        NativeUiAction::CommitFocusedBrowserRow => {
            controller.commit_browser_focus_or_toggle_transport()
        }
        NativeUiAction::ToggleBrowserRowSelection { visible_row } => {
            controller.toggle_browser_row_selection(visible_row)
        }
        NativeUiAction::ExtendBrowserSelectionToRow { visible_row } => {
            controller.extend_browser_selection_to_row(visible_row)
        }
        NativeUiAction::AddRangeBrowserSelection { visible_row } => {
            controller.add_range_browser_selection(visible_row)
        }
        NativeUiAction::ExtendBrowserSelectionFromFocus { delta } => {
            controller.extend_browser_selection_from_focus_action(delta)
        }
        NativeUiAction::AddRangeBrowserSelectionFromFocus { delta } => {
            controller.add_range_browser_selection_from_focus_action(delta)
        }
        NativeUiAction::ToggleFocusedBrowserRowSelection => controller.toggle_focused_selection(),
        NativeUiAction::SelectAllBrowserRows => controller.select_all_browser_rows(),
        NativeUiAction::SetBrowserSearch { query } => controller.set_browser_search(query),
        NativeUiAction::StartBrowserRename => controller.start_browser_rename(),
        NativeUiAction::ConfirmBrowserRename => controller.apply_pending_browser_rename(),
        NativeUiAction::CancelBrowserRename => controller.cancel_browser_rename(),
        NativeUiAction::TagBrowserSelection { target } => {
            controller.tag_selected_browser_target(target.into())
        }
        NativeUiAction::DeleteBrowserSelection => {
            controller.delete_active_browser_selection_action()
        }
        action => return Err(action),
    }
    Ok(())
}

/// Try to dispatch map/native tab actions.
fn apply_map_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SetBrowserTab { map } => controller.set_browser_tab(map),
        NativeUiAction::FocusMapSample { sample_id } => {
            controller.focus_map_sample_and_preview(&sample_id)
        }
        action => return Err(action),
    }
    Ok(())
}

/// Try to dispatch waveform/cursor/zoom native actions.
fn apply_waveform_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SeekWaveform { position_milli } => {
            controller.queue_waveform_seek_milli(position_milli)
        }
        NativeUiAction::SetWaveformCursor { position_milli } => {
            controller.set_waveform_cursor_milli(position_milli)
        }
        NativeUiAction::SetWaveformSelectionRange {
            start_milli,
            end_milli,
        } => controller.set_waveform_selection_range_milli(start_milli, end_milli),
        NativeUiAction::ClearWaveformSelection => controller.clear_waveform_selection_with_focus(),
        NativeUiAction::ZoomWaveform { zoom_in, steps } => {
            controller.zoom_waveform_steps_from_ui(zoom_in, steps)
        }
        NativeUiAction::ZoomWaveformToSelection => {
            controller.zoom_waveform_to_selection_with_focus()
        }
        NativeUiAction::ZoomWaveformFull => controller.zoom_waveform_full_with_focus(),
        action => return Err(action),
    }
    Ok(())
}

/// Try to dispatch prompt/update/progress native actions.
fn apply_prompt_and_update_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SetPromptInput { value } => controller.set_active_prompt_input(value),
        NativeUiAction::ConfirmPrompt => controller.confirm_active_prompt_action(),
        NativeUiAction::CancelPrompt => controller.cancel_active_prompt_action(),
        NativeUiAction::CancelProgress => controller.request_progress_cancel(),
        NativeUiAction::CheckForUpdates => controller.check_for_updates_now(),
        NativeUiAction::OpenUpdateLink => controller.open_update_link(),
        NativeUiAction::InstallUpdate => controller.install_update_and_exit(),
        NativeUiAction::DismissUpdate => controller.dismiss_update_notification(),
        action => return Err(action),
    }
    Ok(())
}

#[cfg(test)]
/// Dispatcher coverage and frame-maintenance regression tests.
mod tests;
