use std::time::Instant;

use super::{
    intent::{PlaybackCommand, PlaybackIntent},
    runtime::PlaybackRuntime,
};
use crate::native_app::app::NativeAppState;

impl NativeAppState {
    pub(in crate::native_app) fn start_playback_current_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_intent(PlaybackIntent::new(start_ratio, end_ratio))
    }

    pub(in crate::native_app) fn start_playback_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> Result<(), String> {
        self.start_playback_intent(PlaybackIntent::with_loop_offset(
            start_ratio,
            end_ratio,
            loop_offset_ratio,
        ))
    }

    pub(in crate::native_app) fn start_playback_intent(
        &mut self,
        intent: PlaybackIntent,
    ) -> Result<(), String> {
        let playback_started_at = Instant::now();
        if !self.waveform.current.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        self.prepare_playback_mode_for_loaded_sample();
        if self.audio.player.is_none() {
            self.audio.pending_playback_start = Some(intent);
            if self.background.audio_open.active().is_none() {
                return Err(String::from("Audio output is starting"));
            }
            return Ok(());
        }
        let command = self.playback_command_for_intent(intent);
        self.execute_playback_command(command, playback_started_at)
    }

    pub(in crate::native_app) fn playback_command_for_intent(
        &self,
        intent: PlaybackIntent,
    ) -> PlaybackCommand {
        let resolved = self.resolve_playback_span(
            intent.start_ratio,
            intent.end_ratio,
            intent.loop_offset_ratio,
        );
        PlaybackCommand::from_intent(intent, resolved, self.audio.loop_playback)
    }

    fn execute_playback_command(
        &mut self,
        command: PlaybackCommand,
        playback_started_at: Instant,
    ) -> Result<(), String> {
        let player = self
            .audio
            .player
            .as_mut()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        PlaybackRuntime::new(
            player,
            self.audio.volume,
            &mut self.audio.output_resolved,
            &mut self.audio.current_playback_span,
            &mut self.waveform.current,
        )
        .execute(command, playback_started_at)
    }
}
