//! Waveform-oriented native action dispatch helpers.
//!
//! The migration controller keeps high-level dispatch ordering in [`super`],
//! while this module narrows waveform routing into smaller surface-specific
//! helpers.

mod editing;
mod navigation;
mod options;
mod selection;

use super::AppController;
use crate::app_core::actions::NativeUiAction;

/// Try to dispatch waveform, zoom, and waveform-selection drag actions.
pub(super) fn apply_waveform_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    let action = match options::apply_waveform_option_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    let action = match selection::apply_waveform_selection_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    let action = match navigation::apply_waveform_navigation_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    editing::apply_waveform_edit_action(controller, action)
}
