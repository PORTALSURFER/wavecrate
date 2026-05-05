//! Apply/persist helpers for audio option changes.
//!
//! This module keeps the player rebuild and status-update flow together so the
//! controller entrypoints stay small and the success/error branches can be tested
//! without depending on live device enumeration.

use super::fallback::{audio_input_fallback_message, audio_output_fallback_message};
use crate::app::controller::{AppController, StatusTone};
use crate::app::state::{ActiveAudioInput, ActiveAudioOutput};

/// Finish an audio-output selection apply after the rebuild result is known.
pub(super) fn apply_audio_selection_result(
    controller: &mut AppController,
    rebuild_result: Result<(), String>,
) {
    controller.ui.audio.selected = controller.settings.audio_output.clone();
    match rebuild_result {
        Ok(()) => {
            controller.ui.audio.output_runtime_error = None;
            let _ = controller.persist_config("Failed to save audio settings");
        }
        Err(err) => {
            controller.ui.audio.output_runtime_error = Some(err.clone());
            controller.set_status(err, StatusTone::Error);
        }
    }
}

/// Refresh the applied output snapshot and fallback warning from the current player.
pub(super) fn update_audio_output_status(controller: &mut AppController) {
    if let Some(player) = controller.audio.player.as_ref() {
        let output = player.borrow().output_details().clone();
        controller.ui.audio.applied = Some(ActiveAudioOutput::from(&output));
        controller.ui.audio.warning =
            audio_output_fallback_message(&controller.settings.audio_output, &output);
        controller.ui.audio.output_runtime_error = None;
    }
}

/// Refresh the applied input snapshot and fallback warning from the resolved input.
pub(super) fn update_audio_input_status(
    controller: &mut AppController,
    input: &crate::audio::ResolvedInput,
) {
    controller.ui.audio.input_applied = Some(ActiveAudioInput::from(input));
    controller.ui.audio.input_warning =
        audio_input_fallback_message(&controller.settings.audio_input, input);
}

/// Rebuild the shared audio player from the current settings and loaded sample.
pub(super) fn rebuild_audio_player(controller: &mut AppController) -> Result<(), String> {
    let loaded_audio = controller.sample_view.wav.loaded_audio.clone();
    controller.audio.player = None;
    let player_rc = match controller.ensure_player() {
        Ok(Some(player_rc)) => player_rc,
        Ok(None) => {
            controller.ui.audio.applied = None;
            let err = String::from("Audio unavailable");
            controller.ui.audio.output_runtime_error = Some(err.clone());
            return Err(err);
        }
        Err(err) => {
            controller.ui.audio.applied = None;
            controller.ui.audio.output_runtime_error = Some(err.clone());
            return Err(err);
        }
    };
    if let Some(audio) = loaded_audio {
        let mut player = player_rc.borrow_mut();
        player.stop();
        player.set_audio(audio.bytes.clone(), audio.duration_seconds);
    }
    update_audio_output_status(controller);
    Ok(())
}
