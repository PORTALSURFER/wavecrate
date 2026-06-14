use super::*;

mod loading;
mod normalized;
mod range;
mod request;
mod telemetry;
mod ui_sync;

use normalized::normalized_audition_gain;
use range::{configure_player_for_playback, plan_playback_range, start_player_range};
use request::{prepare_playback_request, route_pending_load};
use telemetry::{LoadedAudioTelemetry, log_playback_stage};
use ui_sync::{refresh_waveform_image_if_view_stale, sync_playback_ui};

/// Start playback using the current loaded audio or queue loading first if needed.
pub(crate) fn play_audio(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f64>,
) -> Result<(), String> {
    let request = prepare_playback_request(controller, looped, start_override)?;
    if route_pending_load(controller, &request)? {
        return Ok(());
    }

    let player = super::ensure_player(controller)?;
    let Some(player) = player else {
        return Err("Audio unavailable".into());
    };
    configure_player_for_playback(controller, &player);
    let range_plan = plan_playback_range(controller, &request);
    let audition_gain = normalized_audition_gain(
        controller,
        range_plan.audition_start,
        range_plan.audition_end,
    );
    player.borrow_mut().set_playback_gain(audition_gain);

    let telemetry = LoadedAudioTelemetry::from_controller(controller);
    let start_range_started_at = telemetry::stage_timer();
    let span_end = range_plan.span_end;
    let start = start_player_range(&player, range_plan, &request)?;
    log_playback_stage(
        "start_player_range",
        &telemetry,
        start_range_started_at,
        None,
    );
    sync_playback_ui(controller, start, span_end, request.start_override);
    refresh_waveform_image_if_view_stale(controller);
    controller.record_loaded_audio_playback();
    log_playback_stage("play_audio_complete", &telemetry, request.started_at, None);
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
