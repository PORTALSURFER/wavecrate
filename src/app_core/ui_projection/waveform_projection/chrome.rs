use super::channel_view::project_waveform_channel_view_model;
use super::*;
use crate::app_core::state::WaveformSliceBatchProfile;

/// Project waveform chrome labels and action-hint copy.
pub(crate) fn project_waveform_chrome_model(ui: &UiState) -> WaveformChromeModel {
    WaveformChromeModel {
        transport_hint: waveform_transport_hint(ui),
        compare_anchor_available: ui.compare_anchor.is_some(),
        compare_anchor_label: ui.waveform.compare_anchor_label.clone(),
        loop_lock_enabled: ui.waveform.loop_lock_enabled,
        channel_view: project_waveform_channel_view_model(ui.waveform.channel_view),
        normalized_audition_enabled: ui.waveform.normalized_audition_enabled,
        bpm_snap_enabled: ui.waveform.bpm_snap_enabled,
        relative_bpm_grid_enabled: ui.waveform.relative_bpm_grid_enabled,
        transient_snap_enabled: ui.waveform.transient_snap_enabled,
        transient_markers_enabled: ui.waveform.transient_markers_enabled,
        slice_mode_enabled: ui.waveform.slice_mode_enabled,
        exact_duplicate_cleanup_available: ui.waveform.slice_batch_profile
            == WaveformSliceBatchProfile::ExactDuplicateBeats
            && !ui.waveform.slices.is_empty(),
    }
}

/// Build the waveform transport hint text for unlocked and locked loop states.
pub(in crate::app_core::ui_projection) fn waveform_transport_hint(ui: &UiState) -> String {
    match (ui.waveform.loop_lock_enabled, ui.waveform.loop_enabled) {
        (true, true) => String::from("Loop locked on"),
        (true, false) => String::from("Loop locked off"),
        (false, true) => String::from("Loop enabled"),
        (false, false) => String::from("Loop disabled"),
    }
}
