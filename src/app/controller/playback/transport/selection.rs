use super::*;
use crate::app::controller::formatting::format_waveform_bpm_input;
use crate::selection::SelectionEdge;

mod bpm;
mod snapping;

pub(crate) use bpm::scaled_selection_bpm;
use bpm::{
    apply_scaled_bpm, bpm_snap_step, clear_too_small_bpm_selection,
    drag_update_uses_direct_position, smart_scale_target_beats,
};
use snapping::snap_to_transient;

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
    controller.begin_edit_selection_undo("Edit selection");
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
    let range = if drag_update_uses_direct_position(controller, snap_override) {
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

/// Cancel one click-armed playback-selection drag without committing playback effects.
///
/// Plain waveform click-to-seek arms selection creation on press so real drags
/// can extend into a range, but release-without-drag should not leave the
/// controller in a latent drag state or produce an undo entry. This helper
/// clears only that inert armed state and preserves any existing visible
/// selection until the follow-up seek/clear behavior runs.
pub(crate) fn cancel_click_armed_selection_drag(controller: &mut AppController) {
    if !controller.selection_state.range.is_creating() {
        return;
    }
    controller.selection_state.range.finish_drag();
    controller.selection_state.bpm_scale_beats = None;
    controller.selection_state.pending_undo = None;
}

pub(crate) fn finish_edit_selection_drag(controller: &mut AppController) {
    controller.selection_state.edit_range.finish_drag();
    controller.commit_edit_selection_undo();
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
    let before = controller
        .selection_state
        .edit_range
        .range()
        .or(controller.ui.waveform.edit_selection);
    waveform_actions::clear_edit_fade_drag(controller);
    let cleared = controller.selection_state.edit_range.clear();
    if !cleared && controller.ui.waveform.edit_selection.is_none() {
        return;
    }
    controller.selection_state.pending_edit_undo = None;
    controller.apply_edit_selection(None);
    controller.push_edit_selection_undo("Edit selection", before, None);
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

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
