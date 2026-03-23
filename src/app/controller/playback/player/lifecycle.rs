use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Lazily create or return the shared audio player used by playback operations.
pub(crate) fn ensure_player(
    controller: &mut AppController,
) -> Result<Option<Rc<RefCell<AudioPlayer>>>, String> {
    if controller.audio.player.is_none() {
        let mut created = AudioPlayer::from_config(&controller.settings.audio_output)
            .map_err(|err| format!("Audio init failed: {err}"))?;
        created.set_volume(controller.ui.volume);
        created.set_anti_clip_settings(
            controller.settings.controls.anti_clip_fade_enabled,
            controller.settings.controls.anti_clip_fade_ms,
        );
        controller.audio.player = Some(Rc::new(RefCell::new(created)));
        controller.update_audio_output_status();
    }
    Ok(controller.audio.player.clone())
}

/// Queue loop disable after the current cycle boundary to avoid mid-cycle discontinuities.
pub(crate) fn defer_loop_disable_after_cycle(controller: &mut AppController) -> Result<(), String> {
    controller.audio.pending_loop_disable_at = None;
    controller.audio.clear_pending_loop_retarget();
    let Some(player_rc) = ensure_player(controller)? else {
        return Ok(());
    };
    let player_ref = player_rc.borrow();
    let remaining = player_ref.remaining_loop_duration();
    let is_playing = player_ref.is_playing();
    let is_looping = player_ref.is_looping();
    drop(player_ref);

    if !is_playing || !is_looping {
        return Ok(());
    }

    let Some(remaining) = remaining else {
        player_rc.borrow_mut().stop();
        return Ok(());
    };
    if remaining <= Duration::from_millis(5) {
        player_rc.borrow_mut().stop();
        return Ok(());
    }

    controller.audio.pending_loop_disable_at = Some(Instant::now() + remaining);
    Ok(())
}

/// Re-arm loop playback at the next cycle boundary using a new start position.
pub(crate) fn defer_loop_retarget_after_cycle(
    controller: &mut AppController,
    start_override: f64,
) -> Result<bool, String> {
    controller.audio.clear_pending_loop_retarget();
    let Some(player_rc) = ensure_player(controller)? else {
        return Ok(false);
    };
    let player_ref = player_rc.borrow();
    let remaining = player_ref.remaining_loop_duration();
    let is_playing = player_ref.is_playing();
    let is_looping = player_ref.is_looping();
    drop(player_ref);

    if !is_playing || !is_looping {
        return Ok(false);
    }

    let Some(remaining) = remaining else {
        return Ok(false);
    };
    if remaining <= Duration::from_millis(5) {
        return Ok(false);
    }

    controller
        .audio
        .schedule_loop_retarget(Instant::now() + remaining, start_override);
    Ok(true)
}
