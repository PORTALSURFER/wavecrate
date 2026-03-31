use super::*;
use crate::app::controller::playback::waveform_actions::{
    nanos_from_milli, normalized64_from_nanos,
};
use std::time::{Duration, Instant};

/// Debounce window for committing queued waveform seek playback updates.
///
/// This keeps drag-heavy seek interactions cheap by applying the final replay
/// seek shortly after pointer activity settles.
const WAVEFORM_SEEK_COMMIT_DEBOUNCE: Duration = Duration::from_millis(24);

pub(crate) fn seek_to(controller: &mut AppController, position: f64) {
    let looped = controller.ui.waveform.loop_enabled;
    record_play_start(controller, position);
    if let Err(err) = controller.play_audio(looped, Some(position)) {
        controller.set_status(err, StatusTone::Error);
    }
}

/// Start playback immediately from one exact waveform nanounit position.
///
/// This shares the same selection-cleanup semantics as queued click seeks so a
/// plain waveform click outside the active playback selection does not inherit
/// that old selection span and collapse into an inaudible one-frame blip.
pub(crate) fn seek_waveform_nanos(controller: &mut AppController, position_nanos: u32) {
    let clamped = position_nanos.min(1_000_000_000);
    let normalized = normalized64_from_nanos(clamped);
    super::selection::cancel_click_armed_selection_drag(controller);
    clear_selection_for_outside_waveform_seek(controller, normalized);
    seek_to(controller, normalized);
    controller.set_waveform_cursor(normalized as f32);
    controller.focus_waveform_context();
}

/// Queue a waveform seek request and defer playback restart to frame prep.
pub(crate) fn queue_waveform_seek_nanos(controller: &mut AppController, position_nanos: u32) {
    let clamped = position_nanos.min(1_000_000_000);
    super::selection::cancel_click_armed_selection_drag(controller);
    clear_selection_for_outside_waveform_seek(controller, normalized64_from_nanos(clamped));
    controller.set_waveform_cursor_nanos(clamped);
    if should_commit_waveform_seek_immediately(controller) {
        controller.runtime.pending_waveform_seek_nanos = None;
        controller.runtime.pending_waveform_seek_not_before = None;
        let normalized = normalized64_from_nanos(clamped);
        seek_to(controller, normalized);
        controller.set_waveform_cursor(normalized as f32);
        controller.focus_waveform_context();
        return;
    }
    controller.runtime.pending_waveform_seek_nanos = Some(clamped);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() + WAVEFORM_SEEK_COMMIT_DEBOUNCE);
}

/// Queue a waveform seek request and defer playback restart to frame prep.
pub(crate) fn queue_waveform_seek_milli(controller: &mut AppController, position_milli: u16) {
    queue_waveform_seek_nanos(controller, nanos_from_milli(position_milli));
}

/// Record the most recent play start position.
pub(crate) fn record_play_start(controller: &mut AppController, position: f64) {
    record_play_start_with_view_policy(controller, position, false);
}

/// Record the most recent play start position without changing the current waveform view.
pub(crate) fn record_play_start_preserving_view(controller: &mut AppController, position: f64) {
    record_play_start_with_view_policy(controller, position, true);
}

/// Record the most recent play start position and optionally preserve the current waveform view.
fn record_play_start_with_view_policy(
    controller: &mut AppController,
    position: f64,
    preserve_view: bool,
) {
    let clamped = position.clamp(0.0, 1.0) as f32;
    controller.ui.waveform.last_start_marker = Some(clamped);
    if preserve_view {
        if !controller.waveform_ready() {
            return;
        }
        controller.ui.waveform.cursor = Some(clamped);
        controller.ui.waveform.cursor_last_navigation_at = Some(Instant::now());
        return;
    }
    controller.set_waveform_cursor(clamped);
}

/// Flush a deferred waveform seek once its debounce window has elapsed.
pub(crate) fn flush_pending_waveform_seek_commit(controller: &mut AppController) {
    if controller
        .runtime
        .pending_waveform_seek_not_before
        .is_some_and(|deadline| Instant::now() < deadline)
    {
        return;
    }
    controller.runtime.pending_waveform_seek_not_before = None;
    let Some(position_nanos) = controller.runtime.pending_waveform_seek_nanos.take() else {
        return;
    };
    let normalized = normalized64_from_nanos(position_nanos);
    seek_to(controller, normalized);
    controller.set_waveform_cursor(normalized as f32);
    controller.focus_waveform_context();
}

/// Clear the active playback selection when a waveform seek lands outside it.
fn clear_selection_for_outside_waveform_seek(controller: &mut AppController, position: f64) {
    let normalized = position.clamp(0.0, 1.0) as f32;
    let Some(selection) = controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
    else {
        return;
    };
    if waveform_selection_contains_position(selection, normalized) {
        return;
    }
    super::selection::clear_selection(controller);
}

/// Return whether one normalized playback position lands inside a selection.
fn waveform_selection_contains_position(selection: SelectionRange, position: f32) -> bool {
    position >= selection.start() && position <= selection.end()
}

fn should_commit_waveform_seek_immediately(controller: &AppController) -> bool {
    !controller.is_playing()
        && controller.sample_view.wav.loaded_audio.is_some()
        && controller.audio.player.is_some()
}

#[cfg(test)]
mod tests;
