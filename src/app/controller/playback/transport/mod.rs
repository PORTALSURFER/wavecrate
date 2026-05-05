use super::*;

mod looping;
mod playback;
mod seek;
mod selection;
mod volume;

pub(crate) use looping::toggle_loop;
pub(crate) use looping::toggle_loop_lock;
pub(crate) use playback::{
    handle_escape, play_from_current_playhead, play_from_cursor, play_from_start,
    replay_from_last_start, stop_playback_if_active, toggle_play_pause,
};
pub(crate) use seek::{
    flush_pending_waveform_seek_commit, queue_waveform_seek_nanos, record_play_start, seek_to,
    seek_waveform_nanos,
};
pub(crate) use selection::{
    clear_edit_selection, clear_selection, finish_edit_selection_drag, finish_selection_drag,
    is_edit_selection_dragging, is_selection_dragging, scaled_selection_bpm,
    set_edit_selection_range, set_selection_range, set_selection_range_with_smart_scale,
    start_edit_selection_drag, start_selection_drag, start_selection_edge_drag,
    update_edit_selection_drag, update_selection_drag,
};
pub(crate) use volume::{commit_volume_setting, flush_pending_volume_setting, set_volume_live};

const MIN_AUDITION_SELECTION_WIDTH: f32 = 1.0e-6;

/// Return the focused review slice range when keyboard slice review is active.
pub(crate) fn slice_review_audition_range(controller: &AppController) -> Option<SelectionRange> {
    controller
        .slice_review_active()
        .then(|| controller.focused_slice_review_range())
        .flatten()
        .filter(|range| !range.is_empty())
}

/// Return the playback span that should drive audition and looping behavior.
///
/// Slice review takes priority so `Space` and loop playback can audition short
/// silence-split slices without mutating the normal waveform selection. When
/// review is inactive, the controller falls back to the established playback
/// selection rules and BPM minimum guard.
pub(crate) fn playback_audition_selection(controller: &AppController) -> Option<SelectionRange> {
    slice_review_audition_range(controller).or_else(|| {
        controller
            .selection_state
            .range
            .range()
            .or(controller.ui.waveform.selection)
            .filter(|range| range.width() > MIN_AUDITION_SELECTION_WIDTH)
            .filter(|range| super::selection_meets_bpm_min_for_playback(controller, *range))
    })
}
