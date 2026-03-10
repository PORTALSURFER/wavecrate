//! Options-panel projection helpers for the native shell.

use super::*;
use std::path::Path;

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
        trash_folder_label: ui.trash_folder.as_deref().map(project_trash_folder_label),
    }
}

/// Build a concise display label for the configured trash folder.
fn project_trash_folder_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}
