use super::*;
use std::time::Instant;

mod loading;
mod normalized;

use loading::{
    browser_selection_playback_target, queue_or_load_explicit_pending_playback,
    queue_or_load_pending_playback,
};
use normalized::normalized_audition_gain;

/// Start playback using the current loaded audio or queue loading first if needed.
pub(crate) fn play_audio(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f64>,
) -> Result<(), String> {
    if controller.is_recording() {
        return Err("Stop recording before playback".into());
    }
    controller.audio.pending_loop_disable_at = None;
    controller.audio.clear_pending_loop_retarget();
    if controller.has_pending_browser_focus_commit() {
        controller.flush_pending_browser_focus_commit();
    }
    if let Some((source, relative_path)) = browser_selection_playback_target(controller) {
        return queue_or_load_explicit_pending_playback(
            controller,
            &source,
            &relative_path,
            looped,
            start_override,
            false,
        );
    }
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
    refresh_waveform_image_if_view_stale(controller);
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

/// Return the currently selected browser sample when browser focus should start new playback.
///
/// This suppresses redundant reloads when the selected browser row already
/// matches the loaded sample for the active source.
fn configure_player_for_playback(controller: &AppController, player: &Rc<RefCell<AudioPlayer>>) {
    player
        .borrow_mut()
        .set_min_span_seconds(super::super::bpm_min_selection_seconds(controller));
    player
        .borrow()
        .set_edit_fade_state(controller.ui.waveform.edit_selection);
}

fn playback_selection(controller: &AppController) -> Option<SelectionRange> {
    crate::app::controller::playback::transport::playback_audition_selection(controller)
}

fn audition_span(
    selection: Option<SelectionRange>,
    looped: bool,
    start_override: Option<f64>,
    span_end: f32,
) -> (f32, f32) {
    if looped {
        selection
            .as_ref()
            .map(|range| (range.start(), range.end()))
            .unwrap_or((0.0, 1.0))
    } else {
        let span_start = start_override
            .map(|start| start.clamp(0.0, 1.0) as f32)
            .or_else(|| selection.as_ref().map(|range| range.start()))
            .unwrap_or(0.0);
        (span_start, span_end)
    }
}

fn start_player_range(
    player: &Rc<RefCell<AudioPlayer>>,
    selection: Option<SelectionRange>,
    looped: bool,
    start_override: Option<f64>,
    span_end: f32,
) -> Result<f32, String> {
    if looped {
        return start_looped_range(player, selection, start_override);
    }

    let start = start_override
        .map(|start| start.clamp(0.0, 1.0))
        .or_else(|| selection.as_ref().map(|range| f64::from(range.start())))
        .unwrap_or(0.0);
    player
        .borrow_mut()
        .play_range(start, f64::from(span_end), false)?;
    Ok(start as f32)
}

fn start_looped_range(
    player: &Rc<RefCell<AudioPlayer>>,
    selection: Option<SelectionRange>,
    start_override: Option<f64>,
) -> Result<f32, String> {
    if let Some(range) = selection {
        return play_looped_selection(player, range, start_override);
    }
    if let Some(start_pos) = start_override {
        player.borrow_mut().play_full_wrapped_from(start_pos)?;
        return Ok(start_pos as f32);
    }
    player.borrow_mut().play_range(0.0, 1.0, true)?;
    Ok(0.0)
}

fn play_looped_selection(
    player: &Rc<RefCell<AudioPlayer>>,
    range: SelectionRange,
    start_override: Option<f64>,
) -> Result<f32, String> {
    if let Some(start_pos) = start_override
        && start_pos >= f64::from(range.start())
        && start_pos <= f64::from(range.end())
    {
        player.borrow_mut().play_looped_range_from(
            f64::from(range.start()),
            f64::from(range.end()),
            start_pos,
        )?;
        return Ok(start_pos as f32);
    }

    let start = range.start();
    player
        .borrow_mut()
        .play_range(f64::from(range.start()), f64::from(range.end()), true)?;
    Ok(start)
}

fn sync_playback_ui(
    controller: &mut AppController,
    start: f32,
    span_end: f32,
    start_override: Option<f64>,
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

/// Refresh the waveform raster before playback overlays animate over it.
///
/// Zoom and selection interactions can leave a queued or stale waveform image
/// behind the current view window until the next explicit redraw. When playback
/// starts, the selection/playhead overlays immediately begin using the current
/// zoom bounds, so refresh the raster first when those retained inputs drift.
fn refresh_waveform_image_if_view_stale(controller: &mut AppController) {
    let Some(decoded) = controller.sample_view.waveform.decoded.as_ref() else {
        return;
    };
    let Some(render_meta) = controller.sample_view.waveform.render_meta.as_ref() else {
        controller.refresh_waveform_image();
        return;
    };
    let view = controller.ui.waveform.view.clamp();
    let stale_view = (render_meta.view_start - view.start).abs() > f64::EPSILON
        || (render_meta.view_end - view.end).abs() > f64::EPSILON;
    let stale_layout = render_meta.size != controller.sample_view.waveform.size
        || render_meta.samples_len != decoded.frame_count()
        || render_meta.channel_view != controller.ui.waveform.channel_view
        || render_meta.channels != decoded.channels;
    if stale_view || stale_layout || controller.ui.waveform.image.is_none() {
        controller.refresh_waveform_image();
    }
}
