//! Audio-engine and options-panel projection helpers for the native shell.

use super::*;
use std::path::Path;

#[path = "options_panel_projection/audio_engine.rs"]
mod audio_engine;

pub(crate) use audio_engine::{project_audio_engine_chip_model, project_audio_engine_model};

/// Project the native options-panel model from UI state.
pub(crate) fn project_options_panel_model(
    ui: &UiState,
) -> crate::app_core::actions::NativeOptionsPanelModel {
    crate::app_core::actions::NativeOptionsPanelModel {
        visible: ui.options_panel.open,
        default_identifier: ui.options_panel.default_identifier.clone(),
        input_monitoring_enabled: ui.controls.input_monitoring_enabled,
        advance_after_rating_enabled: ui.controls.advance_after_rating,
        destructive_yolo_mode_enabled: ui.controls.destructive_yolo_mode,
        invert_waveform_scroll_enabled: ui.controls.invert_waveform_scroll,
        trash_folder_label: ui.trash_folder.as_deref().map(project_trash_folder_label),
        audio_write_format_label: Some(ui.audio.write_format.summary_label()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_panel_projection_surfaces_audio_write_format() {
        let mut ui = UiState::default();
        ui.audio.write_format.sample_format =
            crate::sample_sources::config::AudioWriteSampleFormat::Pcm24;

        let projected = project_options_panel_model(&ui);

        assert_eq!(
            projected.audio_write_format_label.as_deref(),
            Some("Source rate, 24-bit PCM, Preserve mono/stereo, No dither")
        );
    }
}
