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
use crate::app_core::app_api::controller_state::DerivedNodeId;
use crate::{audio::AudioPlayer, waveform::WaveformRenderer};
use browser_actions::apply_browser_native_ui_action;
use map_actions::apply_map_native_ui_action;
use prompt_update_actions::apply_prompt_and_update_native_ui_action;
use tracing::{error, info};
use waveform_actions::apply_waveform_native_ui_action;

/// Internal frame-preparation plans used by the native bridge.
///
/// The controller still exposes `prepare_native_frame(bool)` as the stable runtime
/// API, but bridge pulls can choose a narrower maintenance lane when the pending
/// state shows that only browser-local work needs to run before projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeFramePreparationPlan {
    /// Run the full maintenance pass before projecting a model pull.
    Full,
    /// Run only the browser/status-safe subset for retained browser pulls.
    BrowserRetainedPull,
    /// Run the animation-only maintenance pass for motion-model pulls.
    MotionOnly,
}

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
    ///
    /// Returns `true` when one dispatcher group accepted the action and `false`
    /// when every dispatcher rejected it as unhandled.
    fn apply_native_ui_action(&mut self, action: NativeUiAction) -> bool;
}

impl AppControllerNativeRuntimeExt for AppController {
    fn prepare_native_frame(&mut self, animation_only: bool) {
        let plan = if animation_only {
            NativeFramePreparationPlan::MotionOnly
        } else {
            NativeFramePreparationPlan::Full
        };
        self.prepare_native_frame_with_plan(plan);
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

    fn apply_native_ui_action(&mut self, action: NativeUiAction) -> bool {
        self.begin_waveform_refresh_batch();
        let action = match apply_transport_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return true;
            }
            Err(action) => action,
        };
        let action = match apply_browser_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return true;
            }
            Err(action) => action,
        };
        let action = match apply_map_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return true;
            }
            Err(action) => action,
        };
        let action = match apply_waveform_native_ui_action(self, action) {
            Ok(()) => {
                self.end_waveform_refresh_batch();
                return true;
            }
            Err(action) => action,
        };
        let handled = match apply_prompt_and_update_native_ui_action(self, action) {
            Ok(()) => true,
            Err(unhandled) => {
                error!(
                    ?unhandled,
                    "native ui action was not handled by any dispatcher group"
                );
                false
            }
        };
        self.end_waveform_refresh_batch();
        handled
    }
}

impl AppController {
    /// Execute one internal native-frame preparation plan.
    pub(crate) fn prepare_native_frame_with_plan(&mut self, plan: NativeFramePreparationPlan) {
        self.poll_background_jobs();
        match plan {
            NativeFramePreparationPlan::Full => {
                self.flush_transport_native_frame_lane();
                self.flush_browser_native_frame_lane();
                self.flush_metadata_native_frame_lane();
                self.flush_waveform_native_frame_lane();
                self.flush_startup_native_frame_lane();
                self.tick_playhead();
                let _ = self.refresh_projection_revision_bus();
                self.update_performance_governor(false);
            }
            NativeFramePreparationPlan::BrowserRetainedPull => {
                self.flush_browser_native_frame_lane();
                let _ = self.refresh_projection_revision_bus();
                self.update_performance_governor(false);
            }
            NativeFramePreparationPlan::MotionOnly => {
                self.record_frame_timing_for_fps();
                if !self.is_playing() {
                    let _ = self.refresh_projection_revision_bus();
                    return;
                }
                self.tick_playhead();
                let _ = self.refresh_projection_revision_bus();
            }
        }
    }

    /// Return whether the bridge may use the browser-retained maintenance lane.
    ///
    /// This path is intentionally conservative: any queued transport, waveform,
    /// metadata, startup, map, or playback-sensitive work keeps the next pull on
    /// the full preparation lane.
    pub(crate) fn can_prepare_browser_retained_pull(&self) -> bool {
        if self.is_playing()
            || self.has_pending_volume_setting_flush()
            || self.has_pending_loaded_duration_metadata_write()
            || self.has_pending_waveform_seek_commit()
            || self.has_pending_waveform_image_refresh()
            || self.has_pending_startup_source_db_maintenance()
            || self.has_pending_startup_audio_refresh()
            || self.is_derived_node_dirty(DerivedNodeId::WaveformState)
            || self.is_derived_node_dirty(DerivedNodeId::MapState)
            || self.is_derived_node_dirty(DerivedNodeId::TransportState)
        {
            return false;
        }
        true
    }

    /// Flush native-frame transport maintenance that can affect persisted runtime state.
    fn flush_transport_native_frame_lane(&mut self) {
        if self.has_pending_volume_setting_flush() {
            self.flush_pending_volume_setting();
        }
    }

    /// Flush native-frame browser/status maintenance needed by retained browser pulls.
    fn flush_browser_native_frame_lane(&mut self) {
        if self.has_pending_age_update_commit() {
            self.flush_pending_age_update_commit();
        }
        if self.has_pending_browser_focus_commit() {
            self.flush_pending_browser_focus_commit();
        }
        if self.has_pending_focused_similarity_highlight_refresh() {
            self.flush_pending_focused_similarity_highlight_refresh();
        }
    }

    /// Flush deferred metadata writes owned by the controller.
    fn flush_metadata_native_frame_lane(&mut self) {
        if self.has_pending_loaded_duration_metadata_write() {
            self.flush_pending_loaded_duration_metadata_write();
        }
    }

    /// Flush waveform work that can change rendered pixels or playback targets.
    fn flush_waveform_native_frame_lane(&mut self) {
        if self.has_pending_waveform_seek_commit() {
            self.flush_pending_waveform_seek_commit();
        }
        if self.has_pending_waveform_image_refresh() {
            self.flush_pending_waveform_image_refresh();
        }
    }

    /// Flush deferred startup work once the runtime is ready to expose it.
    fn flush_startup_native_frame_lane(&mut self) {
        if self.has_pending_startup_source_db_maintenance() {
            self.flush_deferred_startup_source_db_maintenance();
        }
        if self.has_pending_startup_audio_refresh() {
            self.flush_deferred_startup_audio_refresh();
        }
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
        NativeUiAction::PlayCompareAnchor => controller.play_compare_anchor(),
        NativeUiAction::HandleEscape => controller.handle_escape(),
        NativeUiAction::ToggleLoopPlayback => controller.toggle_loop(),
        NativeUiAction::ToggleLoopLock => controller.toggle_loop_lock(),
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
