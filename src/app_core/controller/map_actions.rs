//! Map-tab and map-point UI action dispatch helpers.

use super::AppController;
use crate::app_core::actions::NativeUiAction;

/// Try to dispatch map/UI tab actions.
pub(super) fn apply_map_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserTab {
            map,
        }) => controller.set_browser_tab(map),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FocusMapSample { sample_id },
        ) => controller.focus_map_sample_and_preview(&sample_id),
        action => return Err(action),
    }
    Ok(())
}
