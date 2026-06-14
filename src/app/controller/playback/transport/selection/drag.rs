use super::super::*;
use super::{
    bpm::{
        apply_scaled_bpm, bpm_snap_step, clear_too_small_bpm_selection,
        drag_update_uses_direct_position, smart_scale_target_beats,
    },
    retarget::adjust_playback_after_selection_change,
    snapping::snap_to_transient,
};
use crate::app::controller::formatting::format_waveform_bpm_input;
use crate::selection::SelectionEdge;

/// Begin one new playback-selection drag from the exact pointer anchor.
///
/// The initial anchor is preserved verbatim, but plain press does not create a
/// visible selection yet. BPM/transient snapping is deferred to follow-up drag
/// updates and resize gestures once the pointer actually moves.
pub(crate) fn start_selection_drag(controller: &mut AppController, position: f32) {
    controller.selection_state.skip_next_playback_adjust = false;
    controller.selection_state.bpm_scale_beats = None;
    controller.begin_selection_undo("Selection");
    controller.selection_state.range.arm_new(position);
}

pub(crate) fn start_selection_edge_drag(
    controller: &mut AppController,
    edge: SelectionEdge,
    bpm_scale: bool,
) -> bool {
    if !controller.selection_state.range.begin_edge_drag(edge) {
        return false;
    }
    controller.selection_state.skip_next_playback_adjust = false;
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

pub(crate) fn finish_selection_drag(controller: &mut AppController) {
    controller.selection_state.range.finish_drag();
    let commit_scaled_bpm = controller.selection_state.bpm_scale_beats.take().is_some();
    clear_too_small_bpm_selection(controller);
    if commit_scaled_bpm {
        commit_or_restore_scaled_bpm(controller);
    }
    controller.commit_selection_undo();
    if std::mem::take(&mut controller.selection_state.skip_next_playback_adjust) {
        return;
    }
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

pub(crate) fn set_selection_range(controller: &mut AppController, range: SelectionRange) {
    let had_pending_loop_retarget = controller.audio.pending_loop_retarget.is_some();
    controller.audio.clear_pending_loop_retarget();
    controller.selection_state.range.set_range(Some(range));
    controller.apply_selection(Some(range));

    if had_pending_loop_retarget {
        controller.selection_state.skip_next_playback_adjust = true;
        return;
    }
    controller.selection_state.skip_next_playback_adjust = false;
    if super::retarget::is_playing(controller) && controller.selection_state.range.is_dragging() {
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

pub(crate) fn is_selection_dragging(controller: &AppController) -> bool {
    controller.selection_state.range.is_dragging()
}

pub(crate) fn clear_selection(controller: &mut AppController) {
    controller.audio.clear_pending_loop_retarget();
    controller.selection_state.skip_next_playback_adjust = false;
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

fn commit_or_restore_scaled_bpm(controller: &mut AppController) {
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
