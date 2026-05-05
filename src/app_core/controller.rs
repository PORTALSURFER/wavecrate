//! Backend-neutral controller aliases for migration consumers.
//!
//! These aliases keep native runtime entrypoints stable while the runtime-agnostic
//! controller API remains sourced from the legacy `app` implementation during
//! migration.

/// Native runtime action dispatch orchestration and telemetry helpers.
mod action_dispatch;
/// Browser, source, and folder native action dispatch helpers.
mod browser_actions;
/// Native frame-preparation planning and maintenance helpers.
mod frame_preparation;
/// Map-tab and map-point native action dispatch helpers.
mod map_actions;
/// Prompt, progress, and update native action dispatch helpers.
mod prompt_update_actions;
/// Native-runtime controller startup helpers.
mod startup;
/// Focused waveform-native action dispatch extracted from the main controller shim.
mod waveform_actions;

use crate::app_core::app_api::controller::AppController as LegacyAppController;
pub(crate) use crate::app_core::app_api::controller::build_named_gui_fixture_controller;
pub(crate) use frame_preparation::NativeFramePreparationPlan;
pub use startup::build_native_app_controller;
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
/// Active browser auto-rename row state used by native-shell projection.
pub(crate) type AutoRenameBatchRowState =
    crate::app_core::app_api::controller_state::AutoRenameBatchRowState;

use crate::app_core::actions::{NativeAppModel, NativeUiAction};
#[cfg(test)]
use crate::waveform::WaveformRenderer;
use action_dispatch::apply_native_ui_action;
use browser_actions::apply_browser_native_ui_action;
use map_actions::apply_map_native_ui_action;
use prompt_update_actions::apply_prompt_and_update_native_ui_action;
use waveform_actions::apply_waveform_native_ui_action;

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
        apply_native_ui_action(self, action)
    }
}

#[cfg(test)]
/// Dispatcher coverage and frame-maintenance regression tests.
mod tests;
