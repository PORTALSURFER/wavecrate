use super::{
    diagnostics::log_slow_playback_phase,
    intent::{PlaybackCommand, PlaybackIntent, PlaybackMode},
};
use crate::native_app::app::{NativeAppState, PendingRuntimePlaybackStart};
use std::time::Instant;
use wavecrate::audio::{
    PlaybackRuntimeMode, PlaybackRuntimeRequest, PlaybackRuntimeSource,
    edit_fade_range_from_selection,
};

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
        if self.audio.playback_runtime.is_none() && self.audio.player.is_none() {
            self.audio.pending_playback_start = Some(intent);
            if self.background.audio_open.active().is_none() {
                return Err(String::from("Audio output is starting"));
            }
            return Ok(());
        }
        let command = self.playback_command_for_intent(intent);
        self.submit_playback_command(command, playback_started_at)
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

    fn submit_playback_command(
        &mut self,
        command: PlaybackCommand,
        playback_started_at: Instant,
    ) -> Result<(), String> {
        let request_started_at = Instant::now();
        let request = self.playback_runtime_request(command)?;
        log_slow_playback_phase(
            "playback.start.request_build",
            &self.waveform.current.file_name(),
            "waveform",
            request_started_at,
        );
        let runtime = self
            .audio
            .playback_runtime
            .as_ref()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        let runtime_submit_started_at = Instant::now();
        let request_id = runtime
            .try_play(request)
            .map_err(|err| format!("submit playback request: {err:?}"))?;
        log_slow_playback_phase(
            "playback.start.runtime_try_play",
            &self.waveform.current.file_name(),
            "waveform",
            runtime_submit_started_at,
        );
        let state_update_started_at = Instant::now();
        let playback_start = match command.mode {
            PlaybackMode::Looped { offset_ratio } => offset_ratio,
            PlaybackMode::OneShot => command.resolved.start_ratio,
        };
        self.waveform.current.start_playback(playback_start);
        self.audio.current_playback_span =
            Some((command.resolved.start_ratio, command.resolved.end_ratio));
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart {
            id: request_id,
            path: self.waveform.current.path().display().to_string(),
            span: (command.resolved.start_ratio, command.resolved.end_ratio),
        });
        log_slow_playback_phase(
            "playback.start.state_update",
            &self.waveform.current.file_name(),
            "waveform",
            state_update_started_at,
        );
        log_slow_playback_phase(
            "playback.start.submit_runtime",
            &self.waveform.current.file_name(),
            "waveform",
            playback_started_at,
        );
        Ok(())
    }

    fn playback_runtime_request(
        &self,
        command: PlaybackCommand,
    ) -> Result<PlaybackRuntimeRequest, String> {
        let waveform = &self.waveform.current;
        let duration = waveform.frames() as f32 / waveform.sample_rate().max(1) as f32;
        let source = if let Some(samples) = waveform.playback_samples() {
            PlaybackRuntimeSource::DecodedSamples {
                audio_bytes: waveform.audio_bytes(),
                samples,
                duration,
                sample_rate: waveform.sample_rate(),
                channels: waveform.channels(),
            }
        } else if let Some(cache_file) = waveform.playback_cache_file() {
            PlaybackRuntimeSource::InterleavedF32File {
                path: cache_file.path,
                sample_count: cache_file.sample_count,
                duration,
                sample_rate: waveform.sample_rate(),
                channels: waveform.channels(),
            }
        } else if let Some(path) = waveform.playback_source_file() {
            PlaybackRuntimeSource::AudioFile {
                path,
                duration,
                sample_rate: waveform.sample_rate(),
                channels: waveform.channels(),
            }
        } else {
            PlaybackRuntimeSource::AudioBytes {
                data: waveform.audio_bytes(),
                duration,
                sample_rate: waveform.sample_rate(),
                channels: waveform.channels(),
            }
        };
        let mode = match command.mode {
            PlaybackMode::Looped { offset_ratio } => PlaybackRuntimeMode::Looped {
                start: f64::from(command.resolved.start_ratio),
                end: f64::from(command.resolved.end_ratio),
                offset: f64::from(offset_ratio),
            },
            PlaybackMode::OneShot => PlaybackRuntimeMode::OneShot {
                start: f64::from(command.resolved.start_ratio),
                end: f64::from(command.resolved.end_ratio),
            },
        };
        Ok(PlaybackRuntimeRequest {
            source,
            mode,
            volume: self.audio.volume,
            edit_fade: edit_fade_range_from_selection(waveform.edit_selection()),
        })
    }
}
