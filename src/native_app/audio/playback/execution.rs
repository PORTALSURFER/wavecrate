use super::{
    ActiveSamplePlaybackPlanState, SamplePlaybackAvailableSources, SamplePlaybackPlan,
    diagnostics::log_slow_playback_phase,
    intent::{PlaybackCommand, PlaybackIntent, PlaybackMode},
    plan_sample_playback,
};
use crate::native_app::app::{
    GuiMessage, NativeAppState, PendingPlaybackStart, SamplePlaybackHistory, SamplePlaybackIntent,
    SamplePlaybackRequest,
};
use radiant::prelude as ui;
use std::time::Instant;
use wavecrate::audio::{
    PlaybackMetronomeConfig, PlaybackRuntimeMode, PlaybackRuntimeReplacePolicy,
    PlaybackRuntimeRequest, PlaybackRuntimeSource, PlaybackRuntimeStreamPolicy,
    edit_fade_range_from_selection,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_app) enum SamplePlaybackOutcome {
    Started,
    QueuedLoad,
    Promoted,
    Unavailable,
}

impl NativeAppState {
    pub(in crate::native_app) fn request_sample_playback(
        &mut self,
        request: SamplePlaybackRequest,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Result<SamplePlaybackOutcome, String> {
        let active = self.active_sample_playback_plan_state(request.path.as_str());
        let sources = self.sample_playback_available_sources(request.path.as_str());
        match plan_sample_playback(&request, sources, active) {
            SamplePlaybackPlan::PromoteActiveSession => {
                if self
                    .audio
                    .promote_sample_playback_session_to_waveform(request.path.as_str())
                {
                    return Ok(SamplePlaybackOutcome::Promoted);
                }
            }
            SamplePlaybackPlan::Unavailable => return Ok(SamplePlaybackOutcome::Unavailable),
            SamplePlaybackPlan::QueueLoad | SamplePlaybackPlan::QueuePreviewDecode => {
                self.audio.pending_sample_playback = Some(request.clone());
                self.load_sample_without_autoplay(request.path, context);
                return Ok(SamplePlaybackOutcome::QueuedLoad);
            }
            SamplePlaybackPlan::HandOffPreviewToFullSource
            | SamplePlaybackPlan::SubmitRuntime { .. } => {}
        }

        if !self.waveform.current.has_loaded_sample()
            || self.waveform.instant_preview_active()
            || self.waveform.current.path() != std::path::Path::new(request.path.as_str())
        {
            self.audio.pending_sample_playback = Some(request.clone());
            self.load_sample_without_autoplay(request.path, context);
            return Ok(SamplePlaybackOutcome::QueuedLoad);
        }

        match request.intent {
            SamplePlaybackIntent::RandomAudition => {
                let Some((start_unit, length_unit)) = request.random_units else {
                    return Ok(SamplePlaybackOutcome::Unavailable);
                };
                let span = self.random_audition_span_for_loaded_waveform(
                    super::random_audition::RandomAuditionUnits::new(start_unit, length_unit),
                );
                self.start_random_audition_span(span)?;
            }
            SamplePlaybackIntent::HistoryReplay => {
                self.start_playback_intent_with_history(
                    PlaybackIntent::fixed_region(request.span.0, request.span.1),
                    false,
                )?;
            }
            _ => {
                self.start_playback_intent_with_history(
                    PlaybackIntent::new(request.span.0, request.span.1),
                    request.history.records(),
                )?;
            }
        }
        Ok(SamplePlaybackOutcome::Started)
    }

    fn active_sample_playback_plan_state(&self, path: &str) -> ActiveSamplePlaybackPlanState {
        self.audio
            .sample_playback_session
            .as_ref()
            .map(|session| ActiveSamplePlaybackPlanState {
                same_path: session.request.path == path,
                preview_source: session.source_kind == "preview_samples",
                streamable_source: session.source_kind != "preview_samples",
                waveform_visible: session.request.visibility.updates_waveform_playhead(),
            })
            .unwrap_or_default()
    }

    fn sample_playback_available_sources(&mut self, path: &str) -> SamplePlaybackAvailableSources {
        let loaded_current = self.waveform.current.has_loaded_sample()
            && !self.waveform.instant_preview_active()
            && self.waveform.current.path() == std::path::Path::new(path);
        SamplePlaybackAvailableSources {
            loaded_decoded_samples: loaded_current
                && self.waveform.current.playback_samples().is_some(),
            loaded_f32_cache: loaded_current
                && self.waveform.current.playback_cache_file().is_some(),
            loaded_audio_bytes: loaded_current,
            persisted_f32_descriptor: self
                .waveform
                .cache
                .instant_audition_descriptors
                .contains_key(std::path::Path::new(path)),
            file_backed_audio_descriptor: false,
            preview_clip: self
                .waveform
                .cache
                .preview_audition_clip(std::path::Path::new(path))
                .is_some(),
        }
    }

    pub(in crate::native_app) fn start_playback_current_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_intent(PlaybackIntent::new(start_ratio, end_ratio))
    }

    #[allow(dead_code)]
    pub(in crate::native_app) fn start_playback_fixed_span_without_history(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_intent_with_history(
            PlaybackIntent::fixed_region(start_ratio, end_ratio),
            false,
        )
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
        self.start_playback_intent_with_history(intent, true)
    }

    pub(in crate::native_app) fn start_playback_intent_with_history(
        &mut self,
        intent: PlaybackIntent,
        record_history: bool,
    ) -> Result<(), String> {
        let playback_started_at = Instant::now();
        if !self.waveform.current.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        self.prepare_playback_mode_for_loaded_sample();
        if self.audio.playback_runtime.is_none() && self.audio.player.is_none() {
            self.audio.pending_playback_start = Some(if record_history {
                PendingPlaybackStart::record(intent)
            } else {
                PendingPlaybackStart::skip_history(intent)
            });
            if self.background.audio_open.active().is_none() {
                return Err(String::from("Audio output is starting"));
            }
            return Ok(());
        }
        let command = self.playback_command_for_intent(intent);
        self.submit_playback_command(command, playback_started_at, record_history)
    }

    pub(in crate::native_app) fn playback_command_for_intent(
        &self,
        intent: PlaybackIntent,
    ) -> PlaybackCommand {
        let resolved = self.resolve_playback_span_for_intent(intent);
        PlaybackCommand::from_intent(intent, resolved, self.audio.loop_playback)
    }

    fn submit_playback_command(
        &mut self,
        command: PlaybackCommand,
        playback_started_at: Instant,
        record_history: bool,
    ) -> Result<(), String> {
        let request_started_at = Instant::now();
        let request = self.playback_runtime_request(command)?;
        log_slow_playback_phase(
            "playback.start.request_build",
            &self.waveform.current.file_name(),
            "waveform",
            request_started_at,
        );
        self.log_sample_identity_checkpoint(
            "playback.runtime.request_built",
            "submit_playback_command",
            Some(&self.waveform.current.path()),
            Some(match command.mode {
                PlaybackMode::Looped { .. } => "looped",
                PlaybackMode::OneShot => "one_shot",
            }),
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
        if command.intent.show_start_marker {
            self.waveform.current.start_playback(playback_start);
        } else {
            self.waveform
                .current
                .start_playback_without_marker(playback_start);
        }
        self.audio.clear_playback_progress();
        self.audio.current_playback_span =
            Some((command.resolved.start_ratio, command.resolved.end_ratio));
        let session_request = SamplePlaybackRequest::waveform(
            self.waveform.current.path().display().to_string(),
            (command.resolved.start_ratio, command.resolved.end_ratio),
            SamplePlaybackIntent::WaveformSpan,
            "waveform",
            if record_history {
                SamplePlaybackHistory::Record
            } else {
                SamplePlaybackHistory::Skip
            },
        )
        .with_start_marker(command.intent.show_start_marker);
        let source_kind = self.current_waveform_runtime_source_kind();
        self.audio
            .start_sample_playback_session(session_request, request_id, source_kind);
        if record_history {
            self.record_current_playback_history(
                command.resolved.start_ratio,
                command.resolved.end_ratio,
            );
        }
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
        let (playback_gain, playback_gain_normalization) = self.runtime_playback_gain_for_span(
            command.resolved.start_ratio,
            command.resolved.end_ratio,
        );
        Ok(PlaybackRuntimeRequest {
            source,
            mode,
            stream_policy: PlaybackRuntimeStreamPolicy::full(),
            volume: self.audio.volume,
            playback_gain,
            playback_gain_normalization,
            replace_policy: PlaybackRuntimeReplacePolicy::FadeOutPrevious,
            edit_fade: edit_fade_range_from_selection(waveform.edit_selection()),
            metronome: self.playback_metronome_config_for_span(
                command.resolved.start_ratio,
                command.resolved.end_ratio,
                match command.mode {
                    PlaybackMode::Looped { offset_ratio } => offset_ratio,
                    PlaybackMode::OneShot => command.resolved.start_ratio,
                },
            ),
        })
    }

    pub(in crate::native_app) fn playback_metronome_config_for_span(
        &self,
        playback_start: f32,
        playback_end: f32,
        playback_offset: f32,
    ) -> Option<PlaybackMetronomeConfig> {
        if !self.audio.metronome_enabled {
            return None;
        }
        let (grid_start, grid_end) = self.metronome_grid_span(playback_start, playback_end);
        let total_frames = self.waveform.current.frames().max(1) as u64;
        let grid_start_frame = ratio_to_frame(grid_start, total_frames);
        let grid_end_frame = ratio_to_frame(grid_end, total_frames).max(grid_start_frame + 1);
        let offset_frame = ratio_to_frame(playback_offset, total_frames);
        let cycle_frames = grid_end_frame.saturating_sub(grid_start_frame).max(1);
        let cycle_offset_frames = offset_frame
            .saturating_sub(grid_start_frame)
            .min(cycle_frames.saturating_sub(1));
        Some(
            PlaybackMetronomeConfig::new(u16::from(self.ui.chrome.beat_guide_count))
                .with_cycle(cycle_frames, cycle_offset_frames),
        )
    }

    fn metronome_grid_span(&self, playback_start: f32, playback_end: f32) -> (f32, f32) {
        let playback_start = playback_start.clamp(0.0, 1.0);
        let playback_end = playback_end.clamp(playback_start, 1.0);
        if let Some(selection) = self
            .waveform
            .current
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            && selection_contains_span(selection, playback_start, playback_end)
        {
            return (selection.start(), selection.end());
        }
        (playback_start, playback_end)
    }
}

fn selection_contains_span(
    selection: wavecrate::selection::SelectionRange,
    start: f32,
    end: f32,
) -> bool {
    const EPSILON: f32 = 0.000_1;
    start + EPSILON >= selection.start() && end <= selection.end() + EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::{
        test_support::state::{NativeAppStateFixture, WaveformState},
        waveform::{
            test_decoded_waveform_file_from_mono_samples,
            test_file_backed_waveform_file_from_mono_samples,
        },
    };
    use std::{path::PathBuf, sync::Arc};

    #[test]
    fn runtime_request_uses_summary_gain_for_file_backed_normalized_audition() {
        let file = test_file_backed_waveform_file_from_mono_samples(
            PathBuf::from("normalized-runtime-summary.wav"),
            vec![0.1, 0.1, 0.25, 0.5, 0.1, 0.1, 0.8, 0.8],
        );
        let mut state = NativeAppStateFixture::default().build();
        state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
        state.audio.normalized_audition_enabled = true;

        let command = state.playback_command_for_intent(PlaybackIntent::fixed_region(0.25, 0.5));
        let request = state
            .playback_runtime_request(command)
            .expect("runtime request");

        assert!(matches!(
            request.source,
            PlaybackRuntimeSource::AudioFile { .. }
        ));
        assert!((request.playback_gain - 2.0).abs() < f32::EPSILON);
        assert_eq!(request.playback_gain_normalization, None);
    }

    #[test]
    fn runtime_request_keeps_runtime_normalization_for_decoded_samples() {
        let file = test_decoded_waveform_file_from_mono_samples(
            PathBuf::from("normalized-runtime-decoded.wav"),
            vec![0.1, 0.1, 0.25, 0.5],
        );
        let mut state = NativeAppStateFixture::default().build();
        state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
        state.audio.normalized_audition_enabled = true;

        let command = state.playback_command_for_intent(PlaybackIntent::fixed_region(0.25, 0.5));
        let request = state
            .playback_runtime_request(command)
            .expect("runtime request");

        assert!(matches!(
            request.source,
            PlaybackRuntimeSource::DecodedSamples { .. }
        ));
        assert!((request.playback_gain - 1.0).abs() < f32::EPSILON);
        assert_eq!(
            request.playback_gain_normalization,
            Some(wavecrate::audio::PlaybackRuntimeGainNormalization::new(
                0.25, 0.5
            ))
        );
    }
}

fn ratio_to_frame(ratio: f32, total_frames: u64) -> u64 {
    let ratio = if ratio.is_finite() { ratio } else { 0.0 };
    ((f64::from(ratio.clamp(0.0, 1.0)) * total_frames.max(1) as f64).round() as u64)
        .min(total_frames.max(1))
}
