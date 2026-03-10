use super::*;
use crate::app::controller::formatting::format_waveform_bpm_input;
use crate::selection::SelectionEdge;

const TRANSIENT_SNAP_RADIUS: f32 = 0.01;
const SELECTION_START_SNAP_RADIUS: f32 = 0.01;
const SELECTION_START_SNAP_VIEW_FRACTION: f32 = 0.03;
const SELECTION_START_SNAP_SECONDS: f32 = 0.1;

pub(crate) fn start_selection_drag(controller: &mut AppController, position: f32) {
    controller.selection_state.bpm_scale_beats = None;
    controller.begin_selection_undo("Selection");
    let start = snapped_selection_drag_anchor(controller, position);
    let range = controller.selection_state.range.begin_new(start);
    controller.apply_selection(Some(range));
}

pub(crate) fn start_edit_selection_drag(controller: &mut AppController, position: f32) {
    let _ = controller.commit_edit_selection_fades();
    waveform_actions::clear_edit_fade_drag(controller);
    let start = snap_to_transient(controller, position).unwrap_or(position);
    let range = controller.selection_state.edit_range.begin_new(start);
    controller.apply_edit_selection(Some(range));
}

pub(crate) fn start_selection_edge_drag(
    controller: &mut AppController,
    edge: SelectionEdge,
    bpm_scale: bool,
) -> bool {
    if !controller.selection_state.range.begin_edge_drag(edge) {
        return false;
    }
    controller.begin_selection_undo("Selection");
    controller.selection_state.bpm_scale_beats = if bpm_scale {
        smart_scale_target_beats(controller)
    } else {
        None
    };
    controller.apply_selection(controller.selection_state.range.range());
    true
}

pub(crate) fn update_selection_drag(
    controller: &mut AppController,
    position: f32,
    snap_override: bool,
) {
    let range = if controller.selection_state.bpm_scale_beats.is_some() || snap_override {
        controller.selection_state.range.update_drag(position)
    } else if let Some(step) = bpm_snap_step(controller) {
        controller
            .selection_state
            .range
            .update_drag_snapped(position, step)
    } else {
        let snapped = snap_to_transient(controller, position).unwrap_or(position);
        controller.selection_state.range.update_drag(snapped)
    };
    if let Some(range) = range {
        controller.apply_selection(Some(range));
        if let Some(beats) = controller.selection_state.bpm_scale_beats {
            apply_scaled_bpm(controller, beats, range);
        }
    } else if controller.selection_state.range.range().is_none() {
        controller.apply_selection(None);
    }
}

pub(crate) fn update_edit_selection_drag(
    controller: &mut AppController,
    position: f32,
    snap_override: bool,
) {
    let range = if snap_override {
        controller.selection_state.edit_range.update_drag(position)
    } else {
        let snapped = snap_to_transient(controller, position).unwrap_or(position);
        controller.selection_state.edit_range.update_drag(snapped)
    };
    if let Some(range) = range {
        controller.apply_edit_selection(Some(range));
    } else if controller.selection_state.edit_range.range().is_none() {
        controller.apply_edit_selection(None);
    }
}

pub(crate) fn finish_selection_drag(controller: &mut AppController) {
    controller.selection_state.range.finish_drag();
    let commit_scaled_bpm = controller.selection_state.bpm_scale_beats.take().is_some();
    clear_too_small_bpm_selection(controller);
    if commit_scaled_bpm {
        if controller
            .selection_state
            .range
            .range()
            .or(controller.ui.waveform.selection)
            .is_some()
        {
            if let Some(bpm) = controller.ui.waveform.bpm_value {
                controller.set_bpm_value(bpm);
            }
        } else {
            let persisted_bpm = controller.settings.controls.bpm_value;
            controller.preview_bpm_value(persisted_bpm);
            if let Some(input) = format_waveform_bpm_input(persisted_bpm) {
                controller.ui.waveform.bpm_input = input;
            }
        }
    }
    controller.commit_selection_undo();
    let is_playing = controller
        .audio
        .player
        .as_ref()
        .map(|p| p.borrow().is_playing())
        .unwrap_or(false);
    if !is_playing || !controller.ui.waveform.loop_enabled {
        return;
    }
    let Some(selection) = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
        .filter(|range| super::super::selection_meets_bpm_min_for_playback(controller, *range))
    else {
        return;
    };
    let playhead = controller.ui.waveform.playhead.position;
    let start_override = if playhead >= selection.start() && playhead <= selection.end() {
        Some(playhead)
    } else {
        Some(selection.start())
    };
    if let Err(err) = controller.play_audio(true, start_override) {
        controller.set_status(err, StatusTone::Error);
    }
}

pub(crate) fn finish_edit_selection_drag(controller: &mut AppController) {
    controller.selection_state.edit_range.finish_drag();
}

pub(crate) fn set_selection_range(controller: &mut AppController, range: SelectionRange) {
    controller.selection_state.range.set_range(Some(range));
    controller.apply_selection(Some(range));

    let is_playing = controller
        .audio
        .player
        .as_ref()
        .map(|p| p.borrow().is_playing())
        .unwrap_or(false);

    if is_playing
        && !controller.ui.waveform.loop_enabled
        && let Err(err) = controller.play_audio(false, Some(range.start()))
    {
        controller.set_status(err, StatusTone::Error);
    }
}

pub(crate) fn set_selection_range_with_smart_scale(
    controller: &mut AppController,
    range: SelectionRange,
    beats: f32,
) {
    set_selection_range(controller, range);
    apply_scaled_bpm(controller, beats, range);
}

pub(crate) fn set_edit_selection_range(controller: &mut AppController, range: SelectionRange) {
    waveform_actions::clear_edit_fade_drag(controller);
    controller.selection_state.edit_range.set_range(Some(range));
    controller.apply_edit_selection(Some(range));
}

pub(crate) fn is_selection_dragging(controller: &AppController) -> bool {
    controller.selection_state.range.is_dragging()
}

pub(crate) fn is_edit_selection_dragging(controller: &AppController) -> bool {
    controller.selection_state.edit_range.is_dragging()
}

pub(crate) fn clear_selection(controller: &mut AppController) {
    let before = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection);
    let cleared = controller.selection_state.range.clear();
    if cleared || controller.ui.waveform.selection.is_some() {
        controller.apply_selection(None);
        controller.push_selection_undo("Selection", before, None);
    }
}

pub(crate) fn clear_edit_selection(controller: &mut AppController) {
    waveform_actions::clear_edit_fade_drag(controller);
    let cleared = controller.selection_state.edit_range.clear();
    if cleared || controller.ui.waveform.edit_selection.is_some() {
        controller.apply_edit_selection(None);
    }
}

fn bpm_snap_step(controller: &AppController) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.duration_seconds)?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let step = 60.0 / bpm / duration;
    if step.is_finite() && step > 0.0 {
        Some(step)
    } else {
        None
    }
}

fn clear_too_small_bpm_selection(controller: &mut AppController) {
    let Some(range) = controller.selection_state.range.range() else {
        return;
    };
    if super::super::selection_meets_bpm_min_for_playback(controller, range) {
        return;
    }
    controller.selection_state.range.set_range(None);
    controller.apply_selection(None);
}

fn snap_to_transient(controller: &AppController, position: f32) -> Option<f32> {
    if !controller.ui.waveform.transient_markers_enabled
        || !controller.ui.waveform.transient_snap_enabled
    {
        return None;
    }
    let mut closest = None;
    let mut best_distance = TRANSIENT_SNAP_RADIUS;
    for marker in controller.ui.waveform.transients.iter().copied() {
        let distance = (marker - position).abs();
        if distance <= best_distance {
            best_distance = distance;
            closest = Some(marker);
        }
    }
    closest
}

fn snap_selection_start(controller: &AppController, position: f32) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let radius = selection_start_snap_radius(controller);
    if position.is_finite() && radius.is_finite() && radius > 0.0 && position <= radius {
        Some(0.0)
    } else {
        None
    }
}

fn selection_start_snap_radius(controller: &AppController) -> f32 {
    let mut radius = SELECTION_START_SNAP_RADIUS;
    let view_width = controller.ui.waveform.view.width();
    if view_width.is_finite() && view_width > 0.0 {
        radius = radius.min((view_width * SELECTION_START_SNAP_VIEW_FRACTION as f64) as f32);
    }
    if let Some(duration) = controller.loaded_audio_duration_seconds()
        && duration.is_finite()
        && duration > 0.0
    {
        radius = radius.min(SELECTION_START_SNAP_SECONDS / duration);
    }
    radius
}

fn smart_scale_target_beats(controller: &AppController) -> Option<f32> {
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

fn apply_scaled_bpm(controller: &mut AppController, beats: f32, range: SelectionRange) {
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

fn snapped_selection_drag_anchor(controller: &AppController, position: f32) -> f32 {
    if let Some(step) = bpm_snap_step(controller) {
        return snap_position_to_bpm_step(position, step);
    }
    snap_selection_start(controller, position)
        .or_else(|| snap_to_transient(controller, position))
        .unwrap_or(position)
}

fn snap_position_to_bpm_step(position: f32, step: f32) -> f32 {
    if !position.is_finite() || !step.is_finite() || step <= 0.0 {
        return position;
    }
    let snapped = (position / step).round() * step;
    snapped.clamp(0.0, 1.0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support;

    #[test]
    fn start_selection_drag_snaps_to_zero_with_bpm_snap() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.bpm_snap_enabled = true;

        start_selection_drag(&mut controller, 0.005);

        let range = if let Some(range) = controller.selection_state.range.range() {
            range
        } else {
            panic!("selection range should be initialized");
        };
        assert!((range.start() - 0.0).abs() <= f32::EPSILON);
    }

    #[test]
    fn start_selection_drag_snaps_anchor_to_bpm_step() {
        let (mut controller, source) = test_support::dummy_controller();
        controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: PathBuf::from("snap_anchor.wav"),
            bytes: Vec::new().into(),
            duration_seconds: 4.0,
            sample_rate: 48_000,
        });
        controller.ui.waveform.bpm_snap_enabled = true;
        controller.ui.waveform.bpm_value = Some(120.0);

        start_selection_drag(&mut controller, 0.31);

        let range = controller
            .selection_state
            .range
            .range()
            .expect("selection range should be initialized");
        assert!((range.start() - 0.25).abs() < 1.0e-6);
        assert!((range.end() - 0.25).abs() < 1.0e-6);
    }
}
