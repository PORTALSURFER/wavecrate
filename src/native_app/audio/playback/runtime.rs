use super::{PlaybackCommand, PlaybackMode, log_slow_playback_phase};
use crate::native_app::app::WaveformState;
use std::time::Instant;
use wavecrate::audio::{AudioPlayer, ResolvedOutput, edit_fade_range_from_selection};

pub(super) struct PlaybackRuntime<'a> {
    player: &'a mut AudioPlayer,
    volume: f32,
    output_resolved: &'a mut Option<ResolvedOutput>,
    current_span: &'a mut Option<(f32, f32)>,
    waveform: &'a mut WaveformState,
}

impl<'a> PlaybackRuntime<'a> {
    pub(super) fn new(
        player: &'a mut AudioPlayer,
        volume: f32,
        output_resolved: &'a mut Option<ResolvedOutput>,
        current_span: &'a mut Option<(f32, f32)>,
        waveform: &'a mut WaveformState,
    ) -> Self {
        Self {
            player,
            volume,
            output_resolved,
            current_span,
            waveform,
        }
    }

    pub(super) fn execute(
        &mut self,
        command: PlaybackCommand,
        playback_started_at: Instant,
    ) -> Result<(), String> {
        let start_ratio = command.resolved.start_ratio;
        let end_ratio = command.resolved.end_ratio;
        let source = PlaybackSource::from_waveform(self.waveform);
        let source_kind = source.kind;
        let file_name = self.waveform.file_name();

        self.prepare_output(&file_name, source_kind);
        self.prepare_audio_source(source, &file_name);
        self.prepare_edit_fade(&file_name, source_kind);

        let playback_start = self.start_player(command, &file_name, source_kind)?;
        let waveform_started_at = Instant::now();
        self.waveform.start_playback(playback_start);
        *self.current_span = Some((start_ratio, end_ratio));
        log_slow_playback_phase(
            "playback.start.waveform_state",
            &file_name,
            source_kind,
            waveform_started_at,
        );

        log_slow_playback_phase(
            "playback.start.total",
            &file_name,
            source_kind,
            playback_started_at,
        );
        Ok(())
    }

    fn prepare_output(&mut self, file_name: &str, source_kind: &'static str) {
        let started_at = Instant::now();
        self.player.set_volume(self.volume);
        *self.output_resolved = Some(self.player.output_details().clone());
        log_slow_playback_phase(
            "playback.start.output_setup",
            file_name,
            source_kind,
            started_at,
        );
    }

    fn prepare_audio_source(&mut self, source: PlaybackSource, file_name: &str) {
        let started_at = Instant::now();
        match source.payload {
            PlaybackSourcePayload::DecodedSamples { samples } => {
                self.player.set_audio_samples_with_metadata(
                    self.waveform.audio_bytes(),
                    samples,
                    source.duration,
                    source.sample_rate,
                    source.channels,
                );
            }
            PlaybackSourcePayload::InterleavedF32File { path, sample_count } => {
                self.player.set_interleaved_f32_file_with_metadata(
                    path,
                    sample_count,
                    source.duration,
                    source.sample_rate,
                    source.channels,
                );
            }
            PlaybackSourcePayload::AudioBytes => {
                self.player.set_audio_with_metadata(
                    self.waveform.audio_bytes(),
                    source.duration,
                    source.sample_rate,
                    source.channels,
                );
            }
        }
        log_slow_playback_phase(
            "playback.start.set_audio",
            file_name,
            source.kind,
            started_at,
        );
    }

    fn prepare_edit_fade(&mut self, file_name: &str, source_kind: &'static str) {
        let started_at = Instant::now();
        self.player
            .set_edit_fade_state(edit_fade_range_from_selection(
                self.waveform.edit_selection(),
            ));
        log_slow_playback_phase(
            "playback.start.set_edit_fade",
            file_name,
            source_kind,
            started_at,
        );
    }

    fn start_player(
        &mut self,
        command: PlaybackCommand,
        file_name: &str,
        source_kind: &'static str,
    ) -> Result<f32, String> {
        let started_at = Instant::now();
        let start_ratio = command.resolved.start_ratio;
        let end_ratio = command.resolved.end_ratio;
        let playback_start = match command.mode {
            PlaybackMode::Looped { offset_ratio } => {
                self.player.play_looped_range_from(
                    f64::from(start_ratio),
                    f64::from(end_ratio),
                    f64::from(offset_ratio),
                )?;
                offset_ratio
            }
            PlaybackMode::OneShot => {
                self.player
                    .play_range(f64::from(start_ratio), f64::from(end_ratio), false)?;
                start_ratio
            }
        };
        log_slow_playback_phase(
            "playback.start.player_play",
            file_name,
            source_kind,
            started_at,
        );
        Ok(playback_start)
    }
}

struct PlaybackSource {
    payload: PlaybackSourcePayload,
    duration: f32,
    sample_rate: u32,
    channels: usize,
    kind: &'static str,
}

enum PlaybackSourcePayload {
    DecodedSamples {
        samples: std::sync::Arc<[f32]>,
    },
    InterleavedF32File {
        path: std::path::PathBuf,
        sample_count: u64,
    },
    AudioBytes,
}

impl PlaybackSource {
    fn from_waveform(waveform: &WaveformState) -> Self {
        let duration = waveform.frames() as f32 / waveform.sample_rate().max(1) as f32;
        let sample_rate = waveform.sample_rate();
        let channels = waveform.channels();

        if let Some(samples) = waveform.playback_samples() {
            return Self {
                payload: PlaybackSourcePayload::DecodedSamples { samples },
                duration,
                sample_rate,
                channels,
                kind: "decoded_samples",
            };
        }

        if let Some(cache_file) = waveform.playback_cache_file() {
            return Self {
                payload: PlaybackSourcePayload::InterleavedF32File {
                    path: cache_file.path,
                    sample_count: cache_file.sample_count,
                },
                duration,
                sample_rate,
                channels,
                kind: "interleaved_f32_file",
            };
        }

        Self {
            payload: PlaybackSourcePayload::AudioBytes,
            duration,
            sample_rate,
            channels,
            kind: "audio_bytes",
        }
    }
}
