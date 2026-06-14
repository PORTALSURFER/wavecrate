use super::loading::{
    browser_selection_playback_target, queue_or_load_explicit_pending_playback,
    queue_or_load_pending_playback,
};
use super::telemetry::{self, log_pending_load_stage};
use super::*;

pub(super) struct PlaybackRequest {
    pub(super) looped: bool,
    pub(super) start_override: Option<f64>,
    pub(super) started_at: telemetry::StageStart,
}

pub(super) fn prepare_playback_request(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f64>,
) -> Result<PlaybackRequest, String> {
    let request = PlaybackRequest {
        looped,
        start_override,
        started_at: telemetry::stage_timer(),
    };
    if controller.is_recording() {
        return Err("Stop recording before playback".into());
    }
    controller.audio.pending_loop_disable_at = None;
    controller.audio.clear_pending_loop_retarget();
    if controller.has_pending_browser_focus_commit() {
        controller.flush_pending_browser_focus_commit();
    }
    Ok(request)
}

pub(super) fn route_pending_load(
    controller: &mut AppController,
    request: &PlaybackRequest,
) -> Result<bool, String> {
    if let Some((source, relative_path)) = browser_selection_playback_target(controller) {
        log_pending_load_stage(
            "play_audio_queue_explicit_target",
            request.started_at,
            Some(&source.id),
            Some(&relative_path),
        );
        queue_or_load_explicit_pending_playback(
            controller,
            &source,
            &relative_path,
            request.looped,
            request.start_override,
            false,
        )?;
        return Ok(true);
    }

    if controller.sample_view.wav.loaded_audio.is_none() {
        log_pending_load_stage("play_audio_queue_pending", request.started_at, None, None);
        queue_or_load_pending_playback(controller, request.looped, request.start_override)?;
        return Ok(true);
    }

    Ok(false)
}
