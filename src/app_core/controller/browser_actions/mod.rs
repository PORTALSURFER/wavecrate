//! Browser, source, and folder native action dispatch helpers.
//!
//! The migration controller keeps top-level orchestration in [`super`], while
//! this module narrows browser-side routing into smaller surface-specific
//! helpers.

mod browser;
mod folders;
mod sources;

use super::AppController;
use crate::app_core::actions::NativeUiAction;

/// Try to dispatch browser-and-sources native actions.
pub(super) fn apply_browser_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    let action = match sources::apply_source_and_options_native_ui_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    let action = match folders::apply_folder_native_ui_action(controller, action) {
        Ok(()) => return Ok(()),
        Err(action) => action,
    };
    browser::apply_browser_list_native_ui_action(controller, action)
}
