use super::*;
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

/// Browser action rows used for multi-sample loop/BPM metadata writes.
struct LoopActionTargets {
    /// Primary browser row (loaded sample when visible).
    primary_row: Option<usize>,
    /// Action target paths (selection plus primary when needed).
    paths: Vec<std::path::PathBuf>,
}

/// Resolve action paths targeted by loop metadata updates.
fn loop_action_targets(controller: &mut AppController) -> LoopActionTargets {
    let loaded_path = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.relative_path.clone());
    let primary_row = loaded_path
        .as_ref()
        .and_then(|path| controller.visible_row_for_path(path));
    let paths = primary_row
        .map(|row| controller.browser_action_paths_from_primary(row))
        .unwrap_or_default();
    LoopActionTargets { primary_row, paths }
}

/// Persist loop marker state to selected browser rows or loaded-sample fallback.
fn persist_loop_toggle_markers(controller: &mut AppController, state: LoopToggleState) {
    let action_targets = loop_action_targets(controller);
    if !action_targets.paths.is_empty() {
        persist_browser_loop_markers(controller, &action_targets, state);
    } else {
        persist_loaded_sample_loop_marker(controller, state.loop_enabled);
    }
}

/// Persist loop markers (and initial BPM when enabling) across targeted browser rows.
fn persist_browser_loop_markers(
    controller: &mut AppController,
    action_targets: &LoopActionTargets,
    state: LoopToggleState,
) {
    let primary_row = action_targets.primary_row.unwrap_or(0);
    if let Err(err) = controller.set_loop_marker_browser_sample_paths(
        &action_targets.paths,
        state.loop_enabled,
        primary_row,
    ) {
        tracing::warn!("Failed to update loop markers for browser samples: {err}");
    }
    if state.toggled_to_enabled()
        && let Some(bpm) = controller.ui.waveform.bpm_value
        && bpm.is_finite()
        && bpm > 0.0
        && let Err(err) =
            controller.set_bpm_browser_sample_paths(&action_targets.paths, bpm, primary_row)
    {
        tracing::warn!("Failed to save BPM to browser samples: {err}");
    }
}

/// Persist loop marker state for the loaded sample when no browser rows are actionable.
fn persist_loaded_sample_loop_marker(controller: &mut AppController, loop_enabled: bool) {
    let loop_marker_update =
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .and_then(|loaded_audio| {
                controller
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == loaded_audio.source_id)
                    .map(|source| (source.clone(), loaded_audio.relative_path.clone()))
            });
    if let Some((source, relative_path)) = loop_marker_update
        && let Err(err) =
            controller.set_sample_looped_for_source(&source, &relative_path, loop_enabled, false)
    {
        tracing::warn!("Failed to update loop marker: {err}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support;
    use std::path::Path;
    use std::time::{Duration, Instant};

    #[test]
    fn toggle_loop_enable_clears_pending_disable_deadline() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.audio.pending_loop_disable_at = Some(Instant::now() + Duration::from_secs(1));
        controller.ui.waveform.loop_enabled = false;

        toggle_loop(&mut controller);

        assert!(controller.ui.waveform.loop_enabled);
        assert!(controller.audio.pending_loop_disable_at.is_none());
    }

    #[test]
    fn loop_playback_policy_maps_toggle_transitions() {
        assert_eq!(
            loop_playback_policy(LoopToggleState {
                was_looping: false,
                loop_enabled: true,
            }),
            LoopPlaybackPolicy::RestartIfPlaying
        );
        assert_eq!(
            loop_playback_policy(LoopToggleState {
                was_looping: true,
                loop_enabled: false,
            }),
            LoopPlaybackPolicy::DeferDisableAfterCycle
        );
        assert_eq!(
            loop_playback_policy(LoopToggleState {
                was_looping: false,
                loop_enabled: false,
            }),
            LoopPlaybackPolicy::None
        );
    }

    #[test]
    fn toggle_loop_unlocks_before_persisting_normal_toggle() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.loop_enabled = true;
        controller.set_loop_lock_enabled(true);

        toggle_loop(&mut controller);

        assert!(!controller.ui.waveform.loop_lock_enabled);
        assert!(!controller.ui.waveform.loop_enabled);
    }

    #[test]
    fn toggle_loop_lock_enters_locked_on_from_unlocked() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.loop_enabled = false;
        controller.set_loop_lock_enabled(false);

        toggle_loop_lock(&mut controller);

        assert!(controller.ui.waveform.loop_lock_enabled);
        assert!(controller.ui.waveform.loop_enabled);
    }

    #[test]
    fn toggle_loop_lock_cycles_locked_on_and_off() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.loop_enabled = false;

        toggle_loop_lock(&mut controller);
        assert!(controller.ui.waveform.loop_lock_enabled);
        assert!(controller.ui.waveform.loop_enabled);

        toggle_loop_lock(&mut controller);
        assert!(controller.ui.waveform.loop_lock_enabled);
        assert!(!controller.ui.waveform.loop_enabled);

        toggle_loop_lock(&mut controller);
        assert!(controller.ui.waveform.loop_lock_enabled);
        assert!(controller.ui.waveform.loop_enabled);
    }

    #[test]
    fn toggle_loop_lock_does_not_persist_sample_loop_marker() {
        let (mut controller, source) =
            test_support::prepare_with_source_and_wav_entries(vec![test_support::sample_entry(
                "locked_override.wav",
                crate::sample_sources::Rating::NEUTRAL,
            )]);
        let db = controller.cache_db(&source).expect("db");
        db.set_looped(Path::new("locked_override.wav"), false)
            .expect("seed loop marker");
        controller.sample_view.wav.loaded_audio =
            Some(crate::app::controller::state::audio::LoadedAudio {
                source_id: source.id.clone(),
                root: source.root.clone(),
                relative_path: Path::new("locked_override.wav").to_path_buf(),
                bytes: Vec::new().into(),
                duration_seconds: 1.0,
                sample_rate: 48_000,
            });

        toggle_loop_lock(&mut controller);

        assert_eq!(
            db.looped_for_path(Path::new("locked_override.wav"))
                .expect("load loop marker"),
            Some(false)
        );
    }

    #[test]
    fn loop_restart_start_override_uses_selection_span_when_available() {
        let (mut controller, _source) = test_support::dummy_controller();
        controller.ui.waveform.selection = Some(SelectionRange::new(0.2, 0.6));

        let start_override = loop_restart_start_override(&controller, Some(0.4));

        assert_eq!(start_override, None);
    }

    #[test]
    fn loop_restart_start_override_priority_chain_is_stable() {
        let (mut controller, _source) = test_support::dummy_controller();

        let with_progress = loop_restart_start_override(&controller, Some(0.42));
        assert_eq!(with_progress, Some(f64::from(0.42_f32)));

        controller.ui.waveform.playhead.visible = true;
        controller.ui.waveform.playhead.position = 0.37;
        let with_playhead = loop_restart_start_override(&controller, None);
        assert_eq!(
            with_playhead,
            Some(f64::from(controller.ui.waveform.playhead.position))
        );

        controller.ui.waveform.playhead.visible = false;
        controller.ui.waveform.cursor = Some(0.55);
        controller.ui.waveform.last_start_marker = Some(0.19);
        let with_cursor = loop_restart_start_override(&controller, None);
        assert_eq!(with_cursor, controller.ui.waveform.cursor.map(f64::from));

        controller.ui.waveform.cursor = None;
        let with_last_marker = loop_restart_start_override(&controller, None);
        assert_eq!(
            with_last_marker,
            controller.ui.waveform.last_start_marker.map(f64::from)
        );
    }
}
