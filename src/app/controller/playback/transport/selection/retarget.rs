use super::super::*;

pub(super) fn adjust_playback_after_selection_change(controller: &mut AppController) {
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

pub(super) fn is_playing(controller: &AppController) -> bool {
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
        .filter(|range| {
            super::super::super::selection_meets_bpm_min_for_playback(controller, *range)
        })
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
