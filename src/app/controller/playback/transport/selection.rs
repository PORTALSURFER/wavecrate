use super::*;
use crate::app::controller::formatting::format_waveform_bpm_input;
use crate::selection::SelectionEdge;

const TRANSIENT_SNAP_RADIUS: f32 = 0.01;

/// Begin one new playback-selection drag from the exact pointer anchor.
///
/// The initial anchor is preserved verbatim, but plain press does not create a
/// visible selection yet. BPM/transient snapping is deferred to follow-up drag
/// updates and resize gestures once the pointer actually moves.
pub(crate) fn start_selection_drag(controller: &mut AppController, position: f32) {
    controller.selection_state.bpm_scale_beats = None;
    controller.begin_selection_undo("Selection");
    controller.selection_state.range.arm_new(position);
}

/// Begin one new edit-selection drag from the exact pointer anchor.
///
/// The initial anchor must remain under the pointer for predictable destructive
/// edit painting. Transient snapping still applies once the drag extends.
pub(crate) fn start_edit_selection_drag(controller: &mut AppController, position: f32) {
    let _ = controller.commit_edit_selection_fades();
    waveform_actions::clear_edit_fade_drag(controller);
    let start = position.clamp(0.0, 1.0);
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
    adjust_playback_after_selection_change(controller);
}

pub(crate) fn finish_edit_selection_drag(controller: &mut AppController) {
    controller.selection_state.edit_range.finish_drag();
}

pub(crate) fn set_selection_range(controller: &mut AppController, range: SelectionRange) {
    controller.audio.clear_pending_loop_retarget();
    controller.selection_state.range.set_range(Some(range));
    controller.apply_selection(Some(range));

    if is_playing(controller) && controller.selection_state.range.is_dragging() {
        return;
    }
    adjust_playback_after_selection_change(controller);
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
    controller.audio.clear_pending_loop_retarget();
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

fn adjust_playback_after_selection_change(controller: &mut AppController) {
    if !is_playing(controller) {
        controller.audio.clear_pending_loop_retarget();
        return;
    }
    let Some(selection) = active_playback_selection(controller) else {
        controller.audio.clear_pending_loop_retarget();
        return;
    };
    if !controller.ui.waveform.loop_enabled {
        restart_non_looped_selection_playback(controller, selection);
        return;
    }
    retarget_looped_selection_playback(controller, selection);
}

fn is_playing(controller: &AppController) -> bool {
    controller
        .audio
        .player
        .as_ref()
        .map(|p| p.borrow().is_playing())
        .unwrap_or(false)
}

fn active_playback_selection(controller: &AppController) -> Option<SelectionRange> {
    controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
        .filter(|range| super::super::selection_meets_bpm_min_for_playback(controller, *range))
}

fn restart_non_looped_selection_playback(
    controller: &mut AppController,
    selection: SelectionRange,
) {
    controller.audio.clear_pending_loop_retarget();
    let playhead = current_playhead(controller);
    let start = if playhead_inside_selection(playhead, selection) {
        playhead
    } else {
        selection.start()
    };
    if let Err(err) = controller.play_audio(false, Some(f64::from(start))) {
        controller.set_status(err, StatusTone::Error);
    }
}

fn retarget_looped_selection_playback(controller: &mut AppController, selection: SelectionRange) {
    let playhead = current_playhead(controller);
    if !playhead_inside_selection(playhead, selection) {
        restart_looped_selection_playback(controller, selection.start());
        return;
    }
    schedule_loop_retarget_or_restart(controller, selection.start());
}

fn schedule_loop_retarget_or_restart(controller: &mut AppController, start: f32) {
    match controller.defer_loop_retarget_after_cycle(f64::from(start)) {
        Ok(true) => {}
        Ok(false) => restart_looped_selection_playback(controller, start),
        Err(err) => controller.set_status(err, StatusTone::Error),
    }
}

fn restart_looped_selection_playback(controller: &mut AppController, start: f32) {
    controller.audio.clear_pending_loop_retarget();
    if let Err(err) = controller.play_audio(true, Some(f64::from(start))) {
        controller.set_status(err, StatusTone::Error);
    }
}

fn current_playhead(controller: &AppController) -> f32 {
    controller.ui.waveform.playhead.position.clamp(0.0, 1.0)
}

fn playhead_inside_selection(playhead: f32, selection: SelectionRange) -> bool {
    playhead >= selection.start() && playhead <= selection.end()
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
    fn start_selection_drag_arms_without_creating_visible_selection() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.bpm_snap_enabled = true;

        start_selection_drag(&mut controller, 0.005);

        assert!(controller.selection_state.range.is_dragging());
        assert!(controller.selection_state.range.is_creating());
        assert!(controller.selection_state.range.range().is_none());
        assert!(controller.ui.waveform.selection.is_none());
    }

    #[test]
    fn update_selection_drag_materializes_exact_anchor_before_snapping() {
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
        update_selection_drag(&mut controller, 0.44, false);

        let range = controller
            .selection_state
            .range
            .range()
            .expect("selection range should be initialized");
        assert!((range.start() - 0.31).abs() < 1.0e-6);
        assert!((range.end() - 0.435).abs() < 1.0e-6);
    }

    #[test]
    fn finish_selection_drag_without_motion_keeps_no_selection() {
        let (mut controller, _source) = test_support::dummy_controller();

        start_selection_drag(&mut controller, 0.25);
        finish_selection_drag(&mut controller);

        assert!(!controller.selection_state.range.is_dragging());
        assert!(controller.selection_state.range.range().is_none());
        assert!(controller.ui.waveform.selection.is_none());
    }

    #[test]
    fn start_selection_drag_preserves_existing_visible_selection_until_motion() {
        let (mut controller, _source) = test_support::dummy_controller();
        let existing = SelectionRange::new(0.2, 0.4);
        controller.selection_state.range.set_range(Some(existing));
        controller.apply_selection(Some(existing));

        start_selection_drag(&mut controller, 0.7);

        assert_eq!(controller.ui.waveform.selection, Some(existing));
        assert_eq!(controller.selection_state.range.range(), Some(existing));
        assert!(controller.selection_state.range.is_creating());
    }
}
