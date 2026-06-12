use super::fade_overlay::project_waveform_edit_fade_overlay_model;
use super::image::{effective_waveform_image_signature, project_waveform_image};
use super::selection::{project_waveform_edit_selection_milli, project_waveform_slice_previews};
use super::units::{
    normalized_to_micros, normalized_to_milli, normalized64_to_micros, normalized64_to_milli,
    normalized64_to_nanos,
};
use super::*;

/// Project waveform panel state, selection handles, and cached raster payloads.
///
/// This projection reads controller/UI waveform state and preserves raster reuse
/// by honoring waveform image signatures when available.
pub(crate) fn project_waveform_model(controller: &mut AppController) -> WaveformPanelModel {
    let ui = &controller.ui;
    let view_span = (ui.waveform.view.end - ui.waveform.view.start).clamp(1.0e-9, 1.0);
    let zoom_percent = (100.0 / view_span).round().max(100.0);
    let fade_overlay = project_waveform_edit_fade_overlay_model(ui);
    let projected_playhead = projected_playhead_ratio(controller);
    let waveform_image_signature = effective_waveform_image_signature(controller);
    WaveformPanelModel {
        loaded_label: project_waveform_target_label(ui),
        loading: ui.waveform.loading.is_some(),
        image_rendering: controller.waveform_render_in_progress_for_projection(),
        cursor_milli: ui.waveform.cursor.map(normalized_to_milli),
        playhead_milli: projected_playhead.map(normalized_to_milli),
        playhead_micros: projected_playhead.map(normalized_to_micros),
        selection_milli: ui.waveform.selection.map(|selection| {
            NormalizedRangeModel::from_nanos(
                normalized64_to_nanos(selection.start_f64()),
                normalized64_to_nanos(selection.end_f64()),
            )
        }),
        slices: project_waveform_slice_previews(ui),
        selection_export_flash_nonce: ui.waveform.selection_export_flash_nonce,
        selection_export_failure_flash_nonce: ui.waveform.selection_export_failure_flash_nonce,
        edit_selection_apply_flash_nonce: ui.waveform.edit_selection_apply_flash_nonce,
        edit_selection_milli: project_waveform_edit_selection_milli(ui),
        edit_fade_in_end_milli: fade_overlay.fade_in_end_milli,
        edit_fade_in_end_micros: fade_overlay.fade_in_end_micros,
        edit_fade_in_mute_start_milli: fade_overlay.fade_in_mute_start_milli,
        edit_fade_in_mute_start_micros: fade_overlay.fade_in_mute_start_micros,
        edit_fade_in_curve_milli: fade_overlay.fade_in_curve_milli,
        edit_fade_out_start_milli: fade_overlay.fade_out_start_milli,
        edit_fade_out_start_micros: fade_overlay.fade_out_start_micros,
        edit_fade_out_mute_end_milli: fade_overlay.fade_out_mute_end_milli,
        edit_fade_out_mute_end_micros: fade_overlay.fade_out_mute_end_micros,
        edit_fade_out_curve_milli: fade_overlay.fade_out_curve_milli,
        view_start_milli: normalized64_to_milli(ui.waveform.view.start),
        view_end_milli: normalized64_to_milli(ui.waveform.view.end),
        view_start_micros: normalized64_to_micros(ui.waveform.view.start),
        view_end_micros: normalized64_to_micros(ui.waveform.view.end),
        view_start_nanos: normalized64_to_nanos(ui.waveform.view.start),
        view_end_nanos: normalized64_to_nanos(ui.waveform.view.end),
        beat_step_micros: project_waveform_beat_step_micros(controller),
        bpm_grid_origin_micros: project_waveform_bpm_grid_origin_micros(ui),
        loop_enabled: ui.waveform.loop_enabled,
        tempo_label: ui.waveform.bpm_value.map(|bpm| format!("{bpm:.1} BPM")),
        zoom_label: Some(format!("{zoom_percent:.0}%")),
        waveform_image_signature,
        waveform_image: project_waveform_image(controller, waveform_image_signature),
    }
}

/// Project the user-facing label for the current waveform target.
///
/// When waveform loading is pending, the semantic GUI contract still needs to
/// expose which sample is in flight so scenario assertions and desktop
/// automation can target the correct sample before decode completes.
pub(in crate::app_core::ui_projection) fn project_waveform_target_label(
    ui: &UiState,
) -> Option<String> {
    ui.loaded_wav
        .as_deref()
        .or(ui.waveform.loading.as_deref())
        .map(view_model::sample_display_label)
}

/// Resolve the waveform playhead ratio used by UI projection.
///
/// When transport is actively playing, prefer the live audio-player progress so
/// motion-only redraws are not limited by the UI state's playhead update cadence.
/// Fall back to the last UI playhead snapshot for paused or unavailable players.
pub(in crate::app_core::ui_projection) fn projected_playhead_ratio(
    controller: &AppController,
) -> Option<f32> {
    resolve_projected_playhead_ratio(
        controller.ui.waveform.playhead.visible,
        controller.ui.waveform.playhead.position,
        controller.live_playback_progress(),
    )
}

/// Resolve the preferred playhead ratio from UI and live transport inputs.
///
/// The UI runtime uses this to keep playhead motion smooth while preserving
/// the last known UI snapshot whenever transport is idle or live progress is
/// temporarily unavailable.
pub(in crate::app_core::ui_projection) fn resolve_projected_playhead_ratio(
    playhead_visible: bool,
    ui_ratio: f32,
    live_progress: Option<f32>,
) -> Option<f32> {
    if !playhead_visible {
        return None;
    }
    live_progress
        .filter(|progress| progress.is_finite())
        .map(|progress| progress.clamp(0.0, 1.0))
        .or(Some(ui_ratio.clamp(0.0, 1.0)))
}

/// Project normalized quarter-note beat spacing for BPM-aligned waveform overlays.
fn project_waveform_beat_step_micros(controller: &AppController) -> Option<u32> {
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller.loaded_audio_duration_seconds()?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let normalized_step = 60.0 / bpm / duration;
    (normalized_step.is_finite() && normalized_step > 0.0)
        .then_some(normalized_to_micros(normalized_step))
}

/// Project the active or persisted BPM grid origin into normalized micro space.
fn project_waveform_bpm_grid_origin_micros(ui: &UiState) -> u32 {
    if !ui.waveform.relative_bpm_grid_enabled {
        return 0;
    }
    let origin = ui
        .waveform
        .selection
        .map(|selection| selection.start())
        .unwrap_or(ui.waveform.last_bpm_grid_origin)
        .clamp(0.0, 1.0);
    normalized_to_micros(origin)
}
