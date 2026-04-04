use super::*;
use std::time::Instant;

/// Advance the playhead position based on playback progress.
pub(crate) fn tick_playhead(controller: &mut AppController) {
    controller.poll_background_jobs();
    let Some(player) = controller.audio.player.as_ref().cloned() else {
        if controller.sample_view.waveform.decoded.is_none() {
            hide_waveform_playhead(controller);
        }
        return;
    };
    if let Some(err) = player.borrow_mut().take_error() {
        controller.ui.audio.output_runtime_error = Some(err.clone());
        controller.set_status(format!("Playback error: {}", err), StatusTone::Error);
    }
    if let Some(start_override) = take_due_loop_retarget(controller, &player) {
        controller.audio.pending_loop_retarget = None;
        if let Err(err) = controller.play_audio(true, Some(start_override)) {
            controller.set_status(err, StatusTone::Error);
        }
    }
    let should_resume = should_resume_after_loop_disable(controller, &player);
    if should_resume {
        controller.audio.pending_loop_disable_at = None;
        player.borrow_mut().stop();
    }
    let player_ref = player.borrow();
    let is_playing = player_ref.is_playing();
    let progress = player_ref.progress();
    let is_looping = player_ref.is_looping();
    drop(player_ref);
    update_playhead_from_progress(controller, progress, is_looping, is_playing);
    if !is_playing && controller.sample_view.waveform.decoded.is_none() {
        hide_waveform_playhead(controller);
    }
}

/// Apply one playback-progress sample to the retained waveform playhead state.
pub(crate) fn update_playhead_from_progress(
    controller: &mut AppController,
    progress: Option<f32>,
    is_looping: bool,
    is_playing: bool,
) {
    if let Some(progress) = progress {
        let playhead = &mut controller.ui.waveform.playhead;
        let progress = smooth_progress_after_seek(&mut playhead.recent_seek, progress);
        controller.ui.waveform.playhead.position = progress;
        super::super::playhead_trail::tick_playhead_trail(
            &mut controller.ui.waveform.playhead,
            progress,
            is_looping,
            is_playing,
        );
        if playhead_completed_span(controller, progress, is_looping) {
            hide_waveform_playhead(controller);
        } else {
            controller.ui.waveform.playhead.visible = true;
        }
        return;
    }
    hide_waveform_playhead(controller);
}

/// Hide the waveform playhead/selection overlays when playback is inactive.
pub(crate) fn hide_waveform_playhead(controller: &mut AppController) {
    super::super::playhead_trail::stash_active_trail(&mut controller.ui.waveform.playhead);
    controller.ui.waveform.playhead.visible = false;
    controller.ui.waveform.playhead.active_span_end = None;
    controller.ui.waveform.playhead.recent_seek = None;
}

fn should_resume_after_loop_disable(
    controller: &mut AppController,
    player: &Rc<RefCell<AudioPlayer>>,
) -> bool {
    let player_ref = player.borrow();
    match controller.audio.pending_loop_disable_at {
        Some(_) if !player_ref.is_playing() || !player_ref.is_looping() => {
            controller.audio.pending_loop_disable_at = None;
            false
        }
        Some(deadline) => Instant::now() >= deadline,
        None => false,
    }
}

fn take_due_loop_retarget(
    controller: &mut AppController,
    player: &Rc<RefCell<AudioPlayer>>,
) -> Option<f64> {
    let player_ref = player.borrow();
    controller.audio.take_due_loop_retarget(
        Instant::now(),
        player_ref.is_playing(),
        player_ref.is_looping(),
    )
}

fn playhead_completed_span(controller: &AppController, progress: f32, is_looping: bool) -> bool {
    if is_looping {
        return false;
    }
    let target = controller
        .ui
        .waveform
        .playhead
        .active_span_end
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    progress + playhead_completion_epsilon(controller) >= target
}

fn playhead_completion_epsilon(controller: &AppController) -> f32 {
    let Some(audio) = controller.sample_view.wav.loaded_audio.as_ref() else {
        return super::super::PLAYHEAD_COMPLETION_EPSILON;
    };
    if !audio.duration_seconds.is_finite()
        || audio.duration_seconds <= 0.0
        || audio.sample_rate == 0
    {
        return super::super::PLAYHEAD_COMPLETION_EPSILON;
    }
    let frame_epsilon = 1.0 / (audio.duration_seconds * audio.sample_rate as f32);
    frame_epsilon
        .max(f32::EPSILON)
        .max(super::super::PLAYHEAD_COMPLETION_EPSILON * 0.1)
}

fn smooth_progress_after_seek(
    recent_seek: &mut Option<crate::app::state::PlayheadSeek>,
    progress: f32,
) -> f32 {
    const SEEK_SMOOTH_SECS: f32 = 0.08;
    const SEEK_CLEAR_SECS: f32 = 0.20;
    const EPS: f32 = 1e-6;

    let progress = progress.clamp(0.0, 1.0);
    let Some(seek) = *recent_seek else {
        return progress;
    };

    let elapsed = seek.started_at.elapsed();
    if elapsed.as_secs_f32() >= SEEK_CLEAR_SECS || progress + EPS < seek.position {
        *recent_seek = None;
        return progress;
    }

    if elapsed.as_secs_f32() >= SEEK_SMOOTH_SECS || progress <= seek.position + EPS {
        return progress;
    }

    let t = (elapsed.as_secs_f32() / SEEK_SMOOTH_SECS).clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    seek.position + (progress - seek.position) * eased
}

#[cfg(test)]
pub(crate) fn playhead_completed_span_for_tests(
    controller: &AppController,
    progress: f32,
    is_looping: bool,
) -> bool {
    playhead_completed_span(controller, progress, is_looping)
}

#[cfg(test)]
pub(crate) fn hide_waveform_playhead_for_tests(controller: &mut AppController) {
    hide_waveform_playhead(controller);
}

#[cfg(test)]
mod tests {
    use super::smooth_progress_after_seek;
    use crate::app::state::PlayheadSeek;
    use std::time::{Duration, Instant};

    #[test]
    fn smooth_playhead_progress_after_seek_starts_at_seek_position() {
        let mut seek = Some(PlayheadSeek {
            position: 0.25,
            started_at: Instant::now(),
        });
        let progress = smooth_progress_after_seek(&mut seek, 0.40);
        assert!(progress >= 0.25);
        assert!(progress <= 0.40);
    }

    #[test]
    fn smooth_playhead_progress_after_seek_clears_after_timeout() {
        let mut seek = Some(PlayheadSeek {
            position: 0.25,
            started_at: Instant::now() - Duration::from_millis(500),
        });
        let progress = smooth_progress_after_seek(&mut seek, 0.40);
        assert_eq!(progress, 0.40);
        assert!(seek.is_none());
    }
}
