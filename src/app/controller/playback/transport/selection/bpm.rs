use super::*;
use crate::app::controller::formatting::format_waveform_bpm_input;

pub(super) fn bpm_snap_step(controller: &AppController) -> Option<f32> {
    super::super::super::waveform_bpm_snap_step(controller)
}

pub(super) fn drag_update_uses_direct_position(
    controller: &AppController,
    snap_override: bool,
) -> bool {
    controller.selection_state.bpm_scale_beats.is_some() || snap_override
}

pub(super) fn clear_too_small_bpm_selection(controller: &mut AppController) {
    let Some(range) = controller.selection_state.range.range() else {
        return;
    };
    if super::super::super::selection_meets_bpm_min_for_playback(controller, range) {
        return;
    }
    controller.selection_state.range.set_range(None);
    controller.apply_selection(None);
}

pub(super) fn smart_scale_target_beats(controller: &AppController) -> Option<f32> {
    let duration = controller.loaded_audio_duration_seconds()?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let range = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)?;
    let seconds = range.width() * duration;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    Some(SMART_SCALE_SELECTION_BEATS)
}

pub(super) fn apply_scaled_bpm(controller: &mut AppController, beats: f32, range: SelectionRange) {
    let Some(bpm) = scaled_selection_bpm(controller, beats, range) else {
        return;
    };
    if controller.selection_state.bpm_scale_beats.is_some() {
        controller.preview_bpm_value(bpm);
    } else {
        controller.set_bpm_value(bpm);
    }
    if let Some(input) = format_waveform_bpm_input(bpm) {
        controller.ui.waveform.bpm_input = input;
    }
}

pub(crate) fn scaled_selection_bpm(
    controller: &AppController,
    beats: f32,
    range: SelectionRange,
) -> Option<f32> {
    if !beats.is_finite() || beats <= 0.0 {
        return None;
    }
    let duration = match controller.loaded_audio_duration_seconds() {
        Some(duration) if duration.is_finite() && duration > 0.0 => duration,
        _ => return None,
    };
    let seconds = range.width() * duration;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    let bpm = beats * 60.0 / seconds;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    Some(bpm)
}
