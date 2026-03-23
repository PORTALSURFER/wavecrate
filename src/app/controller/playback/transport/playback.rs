use super::*;
use crate::app::state::FocusContext;

pub(crate) fn replay_from_last_start(controller: &mut AppController) -> bool {
    if let Some(position) = controller.ui.waveform.last_start_marker {
        return restart_playback_at_preserving_view(controller, position);
    }
    if let Some(cursor) = controller.ui.waveform.cursor {
        return restart_playback_at_preserving_view(controller, cursor);
    }
    if controller.ui.waveform.playhead.visible {
        return restart_playback_at_preserving_view(
            controller,
            controller.ui.waveform.playhead.position,
        );
    }
    false
}

pub(crate) fn play_from_start(controller: &mut AppController) -> bool {
    restart_playback_at_preserving_view(controller, play_from_start_position(controller))
}

pub(crate) fn play_from_current_playhead(controller: &mut AppController) -> bool {
    let Some(position) = current_playhead_position(controller) else {
        return false;
    };
    restart_playback_at_preserving_view(controller, position)
}

pub(crate) fn play_from_cursor(controller: &mut AppController) -> bool {
    if !controller.waveform_ready() {
        return false;
    }
    let cursor_from_navigation = match (
        controller.ui.waveform.cursor_last_hover_at,
        controller.ui.waveform.cursor_last_navigation_at,
    ) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(hover), Some(nav)) => nav >= hover,
    };
    if cursor_from_navigation && let Some(cursor) = controller.ui.waveform.cursor {
        return restart_playback_at_preserving_view(controller, cursor);
    }
    replay_from_last_start(controller)
}

pub(crate) fn toggle_play_pause(controller: &mut AppController) {
    let player_rc = match controller.ensure_player() {
        Ok(Some(p)) => p,
        Ok(None) => {
            controller.set_status("Audio unavailable", StatusTone::Error);
            return;
        }
        Err(err) => {
            controller.set_status(err, StatusTone::Error);
            return;
        }
    };
    let _is_playing = player_rc.borrow().is_playing();
    drop(player_rc);
    let _ = controller.play_audio(controller.ui.waveform.loop_enabled, None);
}

pub(crate) fn stop_playback_if_active(controller: &mut AppController) -> bool {
    controller.audio.pending_loop_disable_at = None;
    controller.audio.clear_pending_loop_retarget();
    let Some(player_rc) = controller.audio.player.as_ref() else {
        return false;
    };
    let stopped = {
        let mut player = player_rc.borrow_mut();
        if player.is_playing() {
            player.stop();
            true
        } else {
            false
        }
    };
    if stopped {
        controller.hide_waveform_playhead();
    }
    stopped
}

pub(crate) fn handle_escape(controller: &mut AppController) {
    if controller.cancel_edit_selection_fades() {
        return;
    }
    let selection_active = controller.selection_state.range.range().is_some()
        || controller.ui.waveform.selection.is_some()
        || controller.selection_state.edit_range.range().is_some()
        || controller.ui.waveform.edit_selection.is_some();
    let stopped_playback = stop_playback_if_active(controller);
    if !(selection_active && stopped_playback) {
        super::selection::clear_selection(controller);
        super::selection::clear_edit_selection(controller);
    }
    let had_cursor = controller.ui.waveform.cursor.take().is_some();
    if had_cursor {
        controller.ui.waveform.cursor_last_hover_at = None;
        controller.ui.waveform.cursor_last_navigation_at = None;
        controller.ui.waveform.last_start_marker = Some(0.0);
    }
    if !controller.ui.browser.selection.selected_paths.is_empty() {
        controller.clear_browser_selection();
    }
    if matches!(controller.ui.focus.context, FocusContext::SourceFolders) {
        controller.clear_folder_selection();
    }
}

fn waveform_playback_target_exists(controller: &AppController) -> bool {
    controller.sample_view.wav.selected_wav.is_some()
        || controller.sample_view.wav.loaded_audio.is_some()
}

/// Restart playback at one normalized position without forcing the waveform viewport to recenter.
fn restart_playback_at_preserving_view(controller: &mut AppController, position: f32) -> bool {
    if !waveform_playback_target_exists(controller) {
        return false;
    }
    let clamped = position.clamp(0.0, 1.0);
    super::seek::record_play_start_preserving_view(controller, clamped);
    if let Err(err) = controller.play_audio(controller.ui.waveform.loop_enabled, Some(clamped)) {
        controller.set_status(err, StatusTone::Error);
    }
    true
}

fn current_playhead_position(controller: &AppController) -> Option<f32> {
    if !waveform_playback_target_exists(controller) {
        return None;
    }
    controller
        .ui
        .waveform
        .playhead
        .visible
        .then_some(controller.ui.waveform.playhead.position)
        .or(controller.ui.waveform.cursor)
        .or(controller.ui.waveform.last_start_marker)
        .or(Some(0.0))
}

/// Resolve the playback start used by the global "play from start" action.
///
/// When a real playback selection is active, restarting should audition from the
/// selection start rather than the file head so repeated `Space` presses respect
/// the marked play range.
fn play_from_start_position(controller: &AppController) -> f32 {
    controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
        .filter(|range| range.width() > 0.0)
        .filter(|range| super::super::selection_meets_bpm_min_for_playback(controller, *range))
        .map(|range| range.start())
        .unwrap_or(0.0)
}
