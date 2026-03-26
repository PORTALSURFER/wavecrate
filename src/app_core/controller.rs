//! Backend-neutral controller aliases for migration consumers.
//!
//! These aliases keep native runtime entrypoints stable while the runtime-agnostic
//! controller API remains sourced from the legacy `app` implementation during
//! migration.

/// Browser, source, and folder native action dispatch helpers.
mod browser_actions;
/// Map-tab and map-point native action dispatch helpers.
mod map_actions;
/// Prompt, progress, and update native action dispatch helpers.
mod prompt_update_actions;
/// Focused waveform-native action dispatch extracted from the main controller shim.
mod waveform_actions;

use crate::app_core::app_api::controller::AppController as LegacyAppController;
pub(crate) use crate::app_core::app_api::controller::build_named_gui_fixture_controller;
/// Runtime-facing app controller type used by migration hosts.
pub type AppController = LegacyAppController;
/// Retained browser preload-window cache type used by native-shell projection helpers.
pub(crate) type ProjectedBrowserPreloadWindow =
    crate::app_core::app_api::controller::ProjectedBrowserPreloadWindow;
/// Retained browser-row cache entry used by native-shell projection helpers.
pub(crate) type ProjectedBrowserRowCacheEntry =
    crate::app_core::app_api::controller::ProjectedBrowserRowCacheEntry;
/// Retained map-point cache key used by native-shell projection helpers.
pub(crate) type ProjectedMapPointsCacheKey =
    crate::app_core::app_api::controller::ProjectedMapPointsCacheKey;
/// Retained normalized map-point cache entry used by native-shell projection helpers.
pub(crate) type ProjectedMapPointCacheEntry =
    crate::app_core::app_api::controller::ProjectedMapPointCacheEntry;
/// Retained selected-path lookup cache entry used by native-shell projection helpers.
pub(crate) type ProjectedSelectedPathsLookup =
    crate::app_core::app_api::controller::ProjectedSelectedPathsLookup;
/// Map-point query payload alias used by native-shell map projection.
pub(crate) type UmapPointQuery<'a> = crate::app_core::app_api::controller::UmapPointQuery<'a>;

use std::{cell::RefCell, rc::Rc};

use crate::app_core::actions::{NativeAppModel, NativeUiAction};
use crate::{audio::AudioPlayer, waveform::WaveformRenderer};
use browser_actions::apply_browser_native_ui_action;
use map_actions::apply_map_native_ui_action;
use prompt_update_actions::apply_prompt_and_update_native_ui_action;
use tracing::{error, info};
use waveform_actions::apply_waveform_native_ui_action;

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
        self.poll_background_jobs();
        if self.has_pending_volume_setting_flush() {
            self.flush_pending_volume_setting();
        }
        if self.has_pending_age_update_commit() {
            self.flush_pending_age_update_commit();
        }
        if self.has_pending_focused_similarity_highlight_refresh() {
            self.flush_pending_focused_similarity_highlight_refresh();
        }
        if self.has_pending_loaded_duration_metadata_write() {
            self.flush_pending_loaded_duration_metadata_write();
        }
        if self.has_pending_waveform_seek_commit() {
            self.flush_pending_waveform_seek_commit();
        }
        if self.has_pending_waveform_image_refresh() {
            self.flush_pending_waveform_image_refresh();
        }
        if self.has_pending_startup_source_db_maintenance() {
            self.flush_deferred_startup_source_db_maintenance();
        }
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
        NativeUiAction::PlayFromStart => {
            controller.play_from_start();
        }
        NativeUiAction::PlayFromCurrentPlayhead => {
            controller.play_from_current_playhead();
        }
        NativeUiAction::PlayFromWaveformCursor => {
            controller.play_from_cursor();
        }
        NativeUiAction::PlayWaveformAtPrecise { position_nanos } => {
            controller.seek_waveform_nanos(position_nanos);
        }
        NativeUiAction::ToggleTransport => controller.toggle_play_pause(),
        NativeUiAction::HandleEscape => controller.handle_escape(),
        NativeUiAction::ToggleLoopPlayback => controller.toggle_loop(),
        NativeUiAction::ToggleLoopLock => {
            let enabled = !controller.ui.waveform.loop_lock_enabled;
            controller.set_loop_lock_enabled(enabled);
        }
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

#[cfg(test)]
/// Dispatcher coverage and frame-maintenance regression tests.
mod tests;
