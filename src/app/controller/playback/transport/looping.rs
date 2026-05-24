use super::*;

/// Persistence side effects for loop toggle metadata.
mod persistence;
use persistence::persist_loop_toggle_markers;

/// Loop transport policy tests.
#[cfg(test)]
mod tests;

pub(crate) fn toggle_loop(controller: &mut AppController) {
    if controller.ui.waveform.loop_lock_enabled {
        controller.set_loop_lock_enabled(false);
    }
    let state = flip_loop_toggle_state(controller);
    persist_loop_toggle_markers(controller, state);
    apply_loop_toggle_playback_policy(controller, state);
}

/// Cycle the locked loop override without mutating per-sample loop metadata.
pub(crate) fn toggle_loop_lock(controller: &mut AppController) {
    let state = cycle_loop_lock_state(controller);
    apply_loop_toggle_playback_policy(controller, state);
}

/// Snapshot loop state transition produced by one toggle action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LoopToggleState {
    /// Loop state before toggling.
    was_looping: bool,
    /// Loop state after toggling.
    loop_enabled: bool,
}

impl LoopToggleState {
    /// Return true when the toggle changed loop from disabled to enabled.
    fn toggled_to_enabled(self) -> bool {
        self.loop_enabled && !self.was_looping
    }

    /// Return true when the toggle changed loop from enabled to disabled.
    fn toggled_to_disabled(self) -> bool {
        !self.loop_enabled && self.was_looping
    }
}

/// Loop-playback follow-up behavior after toggling loop mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopPlaybackPolicy {
    /// Restart loop playback if the player is currently active.
    RestartIfPlaying,
    /// Defer loop disable to cycle boundary to avoid abrupt mid-cycle stop.
    DeferDisableAfterCycle,
    /// No additional playback action needed.
    None,
}

/// Flip waveform loop state and return the before/after transition snapshot.
fn flip_loop_toggle_state(controller: &mut AppController) -> LoopToggleState {
    let was_looping = controller.ui.waveform.loop_enabled;
    controller.audio.clear_pending_loop_retarget();
    controller.ui.waveform.loop_enabled = !was_looping;
    LoopToggleState {
        was_looping,
        loop_enabled: controller.ui.waveform.loop_enabled,
    }
}

/// Enter or cycle the locked loop override while preserving sample metadata.
fn cycle_loop_lock_state(controller: &mut AppController) -> LoopToggleState {
    let was_looping = controller.ui.waveform.loop_enabled;
    controller.audio.clear_pending_loop_retarget();
    if !controller.ui.waveform.loop_lock_enabled {
        controller.set_loop_lock_enabled(true);
        controller.ui.waveform.loop_enabled = true;
    } else {
        controller.ui.waveform.loop_enabled = !controller.ui.waveform.loop_enabled;
    }
    LoopToggleState {
        was_looping,
        loop_enabled: controller.ui.waveform.loop_enabled,
    }
}

/// Apply loop-toggle playback policy (restart looping playback or defer disable).
fn apply_loop_toggle_playback_policy(controller: &mut AppController, state: LoopToggleState) {
    match loop_playback_policy(state) {
        LoopPlaybackPolicy::RestartIfPlaying => restart_loop_playback_if_playing(controller),
        LoopPlaybackPolicy::DeferDisableAfterCycle => {
            if let Err(err) = controller.defer_loop_disable_after_cycle() {
                controller.set_status(err, StatusTone::Error);
            }
        }
        LoopPlaybackPolicy::None => {}
    }
}

/// Determine playback follow-up required by one loop toggle transition.
fn loop_playback_policy(state: LoopToggleState) -> LoopPlaybackPolicy {
    if state.toggled_to_enabled() {
        LoopPlaybackPolicy::RestartIfPlaying
    } else if state.toggled_to_disabled() {
        LoopPlaybackPolicy::DeferDisableAfterCycle
    } else {
        LoopPlaybackPolicy::None
    }
}

/// Restart loop playback from progress/cursor/playhead context when currently playing.
fn restart_loop_playback_if_playing(controller: &mut AppController) {
    controller.audio.pending_loop_disable_at = None;
    let Some(player_rc) = controller.audio.player.as_ref().cloned() else {
        return;
    };
    let (is_playing, progress) = {
        let player_ref = player_rc.borrow();
        (player_ref.is_playing(), player_ref.progress())
    };
    if !is_playing {
        return;
    }
    let start_override = loop_restart_start_override(controller, progress);
    if let Err(err) = controller.play_audio(true, start_override) {
        controller.set_status(err, StatusTone::Error);
    }
}

/// Compute restart start position for loop-enabled playback based on active selection and cursor state.
fn loop_restart_start_override(controller: &AppController, progress: Option<f32>) -> Option<f64> {
    if has_loop_playback_selection(controller) {
        return None;
    }
    progress.map(f64::from).or_else(|| {
        if controller.ui.waveform.playhead.visible {
            Some(f64::from(controller.ui.waveform.playhead.position))
        } else {
            controller.ui.waveform.cursor.map(f64::from).or(controller
                .ui
                .waveform
                .last_start_marker
                .map(f64::from))
        }
    })
}

/// Return true when a valid playback selection should drive loop restart position.
fn has_loop_playback_selection(controller: &AppController) -> bool {
    super::playback_audition_selection(controller).is_some()
}
