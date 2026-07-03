//! Backend-neutral controller facade for migration consumers.
//!
//! The facade keeps UI runtime entrypoints stable while the current legacy
//! controller remains the backend implementation during migration.

/// UI runtime action dispatch orchestration and telemetry helpers.
mod action_dispatch;
/// Browser, source, and folder UI action dispatch helpers.
mod browser_actions;
/// UI frame-preparation planning and maintenance helpers.
mod frame_preparation;
/// Map-tab and map-point UI action dispatch helpers.
mod map_actions;
/// Prompt, progress, and update UI action dispatch helpers.
mod prompt_update_actions;
/// App-core-owned runtime facade over the retained legacy controller backend.
mod runtime_facade;
/// UI-runtime controller startup helpers.
mod startup;
/// Focused waveform-UI action dispatch extracted from the main controller shim.
mod waveform_actions;

pub(crate) use frame_preparation::UiFramePreparationPlan;
pub use runtime_facade::AppController;
pub use startup::build_ui_app_controller;

use crate::app_core::actions::{NativeAppModel, NativeUiAction};
#[cfg(test)]
use crate::waveform::WaveformRenderer;
use action_dispatch::apply_ui_action;
use browser_actions::apply_browser_ui_action;
use map_actions::apply_map_ui_action;
use prompt_update_actions::apply_prompt_and_update_ui_action;
use waveform_actions::apply_waveform_ui_action;

/// Backend-neutral UI-runtime orchestration helpers exposed by the facade.
pub trait AppControllerUiRuntimeExt {
    /// Apply per-frame controller maintenance before projecting the UI model.
    ///
    /// `animation_only` enables a minimal update path for motion-only frames, for
    /// example when only waveform cursor or playhead values changed.
    fn prepare_ui_frame(&mut self, animation_only: bool);

    /// Project the current controller state into a UI runtime app model.
    fn project_ui_app_model(&mut self) -> NativeAppModel;

    /// Project motion-only fields for incremental animation updates.
    fn project_ui_motion_model(&mut self) -> crate::app_core::actions::NativeMotionModel;

    /// Persist full configuration during UI runtime shutdown.
    fn persist_ui_exit_config(&self) -> Result<(), String>;

    /// Apply a UI runtime UI action to the controller.
    ///
    /// Returns `true` when one dispatcher group accepted the action and `false`
    /// when every dispatcher rejected it as unhandled.
    fn apply_ui_action(&mut self, action: NativeUiAction) -> bool;
}

impl AppControllerUiRuntimeExt for AppController {
    fn prepare_ui_frame(&mut self, animation_only: bool) {
        let plan = if animation_only {
            UiFramePreparationPlan::MotionOnly
        } else {
            UiFramePreparationPlan::Full
        };
        self.prepare_ui_frame_with_plan(plan);
    }

    fn project_ui_app_model(&mut self) -> NativeAppModel {
        crate::app_core::ui_projection::project_app_model(self)
    }

    fn project_ui_motion_model(&mut self) -> crate::app_core::actions::NativeMotionModel {
        crate::app_core::ui_projection::project_motion_model(self)
    }

    fn persist_ui_exit_config(&self) -> Result<(), String> {
        self.save_full_config()
            .map_err(|err| format!("Failed to persist config on UI runtime exit: {err}"))
    }

    fn apply_ui_action(&mut self, action: NativeUiAction) -> bool {
        apply_ui_action(self, action)
    }
}

#[cfg(test)]
/// Dispatcher coverage and frame-maintenance regression tests.
mod tests;
