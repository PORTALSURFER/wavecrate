use super::*;
use std::time::Instant;

/// Start playback using the current loaded audio or queue loading first if needed.
pub(crate) fn play_audio(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f32>,
) -> Result<(), String> {
    if controller.is_recording() {
        return Err("Stop recording before playback".into());
    }
    controller.audio.pending_loop_disable_at = None;
    if controller.sample_view.wav.loaded_audio.is_none() {
        return queue_or_load_pending_playback(controller, looped, start_override);
    }

    let player = super::ensure_player(controller)?;
    let Some(player) = player else {
        return Err("Audio unavailable".into());
    };
    configure_player_for_playback(controller, &player);
    let selection = playback_selection(controller);
    let span_end = selection.as_ref().map(|r| r.end()).unwrap_or(1.0);
    let (audition_start, audition_end) = audition_span(selection, looped, start_override, span_end);
    let audition_gain = normalized_audition_gain(controller, audition_start, audition_end);
    player.borrow_mut().set_playback_gain(audition_gain);
    let start = start_player_range(&player, selection, looped, start_override, span_end)?;
    sync_playback_ui(controller, start, span_end, start_override);
    controller.record_loaded_audio_playback();
    Ok(())
}

/// Return true if audio playback is currently active.
pub(crate) fn is_playing(controller: &AppController) -> bool {
    controller
        .audio
        .player
        .as_ref()
        .map(|p| p.borrow().is_playing())
        .unwrap_or(false)
}

/// Return live player progress while transport is actively running.
///
/// This accessor lets projection paths sample the transport clock directly so
/// animation-only redraws can avoid visible stepping between UI-state updates.
pub(crate) fn live_progress(controller: &AppController) -> Option<f32> {
    let player = controller.audio.player.as_ref()?;
    let player = player.borrow();
    player
        .is_playing()
        .then(|| player.progress())
        .flatten()
        .filter(|progress| progress.is_finite())
        .map(|progress| progress.clamp(0.0, 1.0))
}

fn queue_or_load_pending_playback(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f32>,
) -> Result<(), String> {
    if let Some(pending) = controller.runtime.jobs.pending_audio() {
        controller
            .runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: pending.source_id,
                relative_path: pending.relative_path,
                looped,
                start_override,
            }));
        controller.set_status("Loading audio…", StatusTone::Busy);
        return Ok(());
    }

    let Some(selected) = controller.sample_view.wav.selected_wav.clone() else {
        return Err("Load a .wav file first".into());
    };
    let Some(source) = controller.current_source() else {
        return Err("Load a .wav file first".into());
    };
    let pending_playback = PendingPlayback {
        source_id: source.id.clone(),
        relative_path: selected.clone(),
        looped,
        start_override,
    };
    controller
        .runtime
        .jobs
        .set_pending_playback(Some(pending_playback.clone()));
    controller.queue_audio_load_for(
        &source,
        &selected,
        AudioLoadIntent::Selection,
        Some(pending_playback),
    )?;
    controller.set_status(format!("Loading {}", selected.display()), StatusTone::Busy);
    Ok(())
}

fn configure_player_for_playback(controller: &AppController, player: &Rc<RefCell<AudioPlayer>>) {
    player
        .borrow_mut()
        .set_min_span_seconds(super::super::bpm_min_selection_seconds(controller));
    player
        .borrow()
        .set_edit_fade_state(controller.ui.waveform.edit_selection);
}

fn playback_selection(controller: &AppController) -> Option<SelectionRange> {
    controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
        .filter(|range| range.width() > super::PLAYBACK_SELECTION_MIN_WIDTH)
        .filter(|range| super::super::selection_meets_bpm_min_for_playback(controller, *range))
}

fn audition_span(
    selection: Option<SelectionRange>,
    looped: bool,
    start_override: Option<f32>,
    span_end: f32,
) -> (f32, f32) {
    if looped {
        selection
            .as_ref()
            .map(|range| (range.start(), range.end()))
            .unwrap_or((0.0, 1.0))
    } else {
        let span_start = start_override
            .or_else(|| selection.as_ref().map(|range| range.start()))
            .unwrap_or(0.0);
        (span_start, span_end)
    }
}

fn start_player_range(
    player: &Rc<RefCell<AudioPlayer>>,
    selection: Option<SelectionRange>,
    looped: bool,
    start_override: Option<f32>,
    span_end: f32,
) -> Result<f32, String> {
    if looped {
        return start_looped_range(player, selection, start_override);
    }

    let start = start_override
        .or_else(|| selection.as_ref().map(|range| range.start()))
        .unwrap_or(0.0);
    player.borrow_mut().play_range(start, span_end, false)?;
    Ok(start)
}

fn start_looped_range(
    player: &Rc<RefCell<AudioPlayer>>,
    selection: Option<SelectionRange>,
    start_override: Option<f32>,
) -> Result<f32, String> {
    if let Some(range) = selection {
        return play_looped_selection(player, range, start_override);
    }
    if let Some(start_pos) = start_override {
        player.borrow_mut().play_full_wrapped_from(start_pos)?;
        return Ok(start_pos);
    }
    player.borrow_mut().play_range(0.0, 1.0, true)?;
    Ok(0.0)
}

fn play_looped_selection(
    player: &Rc<RefCell<AudioPlayer>>,
    range: SelectionRange,
    start_override: Option<f32>,
) -> Result<f32, String> {
    if let Some(start_pos) = start_override
        && start_pos >= range.start()
        && start_pos <= range.end()
    {
        player
            .borrow_mut()
            .play_looped_range_from(range.start(), range.end(), start_pos)?;
        return Ok(start_pos);
    }

    let start = range.start();
    player
        .borrow_mut()
        .play_range(range.start(), range.end(), true)?;
    Ok(start)
}

fn sync_playback_ui(
    controller: &mut AppController,
    start: f32,
    span_end: f32,
    start_override: Option<f32>,
) {
    controller.ui.waveform.playhead.active_span_end = Some(span_end.clamp(0.0, 1.0));
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = start;
    super::super::playhead_trail::start_or_seek_trail(
        &mut controller.ui.waveform.playhead,
        start,
        start_override.is_some(),
    );
    if start_override.is_some() {
        controller.ui.waveform.playhead.recent_seek = Some(crate::app::state::PlayheadSeek {
            position: start,
            started_at: Instant::now(),
        });
    }
}

fn normalized_audition_gain(controller: &AppController, start: f32, end: f32) -> f32 {
    if !controller.ui.waveform.normalized_audition_enabled {
        return 1.0;
    }
    let Some(decoded) = controller.sample_view.waveform.decoded.as_ref() else {
        return 1.0;
    };
    let Some(peak) = decoded.max_abs_in_span(start, end) else {
        return 1.0;
    };
    if peak <= f32::EPSILON {
        return 1.0;
    }
    1.0 / peak
}
