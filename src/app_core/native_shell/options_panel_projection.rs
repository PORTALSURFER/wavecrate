//! Options-panel projection helpers for the native shell.

use super::*;

/// Project the native options-panel model from UI state.
pub(crate) fn project_options_panel_model(
    ui: &UiState,
) -> crate::app_core::actions::NativeOptionsPanelModel {
    crate::app_core::actions::NativeOptionsPanelModel {
        visible: ui.options_panel.open,
        input_monitoring_enabled: ui.controls.input_monitoring_enabled,
        advance_after_rating_enabled: ui.controls.advance_after_rating,
        destructive_yolo_mode_enabled: ui.controls.destructive_yolo_mode,
        invert_waveform_scroll_enabled: ui.controls.invert_waveform_scroll,
    }
}
