use rand::Rng;
use span::ResolvedPlaybackSpan;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use wavecrate::audio::AudioPlayer;

use super::{
    GuiAppState, GuiMessage, PLAYBACK_START_ACTIVE_SOURCE_GRACE, PendingPlaybackStart,
    WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID, emit_gui_action, sample_path_label,
};
use radiant::prelude as ui;

mod loop_control;
mod progress;
mod span;

const RANDOM_AUDITION_SECONDS: f32 = 4.0;

impl GuiAppState {
    pub(super) fn play_selected_sample(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        if let Some(path) = self.folder_browser.selected_file_id()
            && self.waveform.path() != Path::new(path)
        {
            let label = sample_path_label(path);
            emit_gui_action(
                "playback.play_selected_sample",
                Some("transport"),
                Some(&label),
                "load_queued",
                started_at,
                None,
            );
            self.select_sample(path.to_string(), context);
            return;
        }
        let (start, end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        match self.start_playback_current_span(start, end) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn play_waveform_from_ratio(&mut self, start_ratio: f32) {
        let started_at = Instant::now();
        match self.start_playback_current_span(start_ratio, 1.0) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status =
                    format!("Playing {} from {:.1}%", file_name, start_ratio * 100.0);
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn play_random_sample_range(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let mut rng = rand::rng();
        self.play_random_sample_range_with_unit(rng.random::<f32>(), context);
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(in crate::gui_app) fn play_random_sample_range_with_unit(
        &mut self,
        unit: f32,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if let Some(path) = self.folder_browser.selected_file_id()
            && self.waveform.path() != Path::new(path)
        {
            let label = sample_path_label(path);
            emit_gui_action(
                "playback.play_random_sample_range",
                Some("transport"),
                Some(&label),
                "load_queued",
                started_at,
                None,
            );
            self.select_sample(path.to_string(), context);
            return;
        }
        let file_name = self.waveform.file_name();
        let (start, end) = random_audition_span_for_unit(self.waveform.duration_seconds(), unit);
        let was_looping = self.loop_playback;
        self.loop_playback = false;

        match self.start_playback_current_span(start, end) {
            Ok(()) => {
                self.sample_status =
                    format!("Random audition {file_name} from {:.1}%", start * 100.0);
                emit_gui_action(
                    "playback.play_random_sample_range",
                    Some("transport"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.loop_playback = was_looping;
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_random_sample_range",
                    Some("transport"),
                    Some(&file_name),
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn stop_playback(&mut self) {
        let started_at = Instant::now();
        if let Some(player) = self.audio_player.as_mut() {
            player.stop();
        }
        self.waveform.stop_playback();
        self.current_playback_span = None;
        let file_name = self.waveform.file_name();
        self.sample_status = format!("Stopped {file_name}");
        emit_gui_action(
            "playback.stop",
            Some("transport"),
            Some(&file_name),
            "success",
            started_at,
            None,
        );
    }

    pub(super) fn start_playback_current_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_span(start_ratio, end_ratio, None)
    }

    pub(super) fn start_playback_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> Result<(), String> {
        let playback_started_at = Instant::now();
        if !self.waveform.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        if self.audio_player.is_none() {
            self.pending_playback_start = Some(PendingPlaybackStart {
                start_ratio,
                end_ratio,
                loop_offset_ratio,
            });
            if self.audio_open_task.active().is_none() {
                return Err(String::from("Audio output is starting"));
            }
            return Ok(());
        }
        let playback_span = self.resolve_playback_span(start_ratio, end_ratio, loop_offset_ratio);
        let start_ratio = playback_span.start_ratio;
        let end_ratio = playback_span.end_ratio;
        let duration = self.waveform.frames() as f32 / self.waveform.sample_rate().max(1) as f32;
        let player = self
            .audio_player
            .as_mut()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        let source_kind = if self.waveform.playback_samples().is_some() {
            "decoded_samples"
        } else {
            "audio_bytes"
        };
        let file_name = self.waveform.file_name();
        let output_setup_started_at = Instant::now();
        player.set_volume(self.volume);
        self.audio_output_resolved = Some(player.output_details().clone());
        log_slow_playback_phase(
            "playback.start.output_setup",
            &file_name,
            source_kind,
            output_setup_started_at,
        );
        let metadata_started_at = Instant::now();
        if let Some(samples) = self.waveform.playback_samples() {
            player.set_audio_samples_with_metadata(
                self.waveform.audio_bytes(),
                samples,
                duration,
                self.waveform.sample_rate(),
                self.waveform.channels(),
            );
        } else {
            player.set_audio_with_metadata(
                self.waveform.audio_bytes(),
                duration,
                self.waveform.sample_rate(),
                self.waveform.channels(),
            );
        }
        log_slow_playback_phase(
            "playback.start.set_audio",
            &file_name,
            source_kind,
            metadata_started_at,
        );
        let fade_started_at = Instant::now();
        player.set_edit_fade_state(self.waveform.edit_selection());
        log_slow_playback_phase(
            "playback.start.set_edit_fade",
            &file_name,
            source_kind,
            fade_started_at,
        );
        let play_started_at = Instant::now();
        let playback_start = if self.loop_playback {
            player.play_looped_range_from(
                f64::from(start_ratio),
                f64::from(end_ratio),
                f64::from(playback_span.offset_ratio),
            )?;
            playback_span.offset_ratio
        } else {
            player.play_range(f64::from(start_ratio), f64::from(end_ratio), false)?;
            start_ratio
        };
        log_slow_playback_phase(
            "playback.start.player_play",
            &file_name,
            source_kind,
            play_started_at,
        );
        let waveform_started_at = Instant::now();
        self.waveform.start_playback(playback_start);
        self.current_playback_span = Some((start_ratio, end_ratio));
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

    pub(super) fn resolve_playback_span(
        &self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> ResolvedPlaybackSpan {
        let requested_start = start_ratio.clamp(0.0, 1.0);
        let requested_end = end_ratio.clamp(requested_start, 1.0);
        if !self.loop_playback {
            return ResolvedPlaybackSpan {
                start_ratio: requested_start,
                end_ratio: requested_end,
                offset_ratio: requested_start,
            };
        }

        let (loop_start, loop_end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        let start_ratio = loop_start.clamp(0.0, 1.0);
        let end_ratio = loop_end.clamp(start_ratio, 1.0);
        let requested_offset = loop_offset_ratio.unwrap_or(requested_start).clamp(0.0, 1.0);
        let offset_ratio = if (start_ratio..=end_ratio).contains(&requested_offset) {
            requested_offset
        } else {
            start_ratio
        };

        ResolvedPlaybackSpan {
            start_ratio,
            end_ratio,
            offset_ratio,
        }
    }
}

pub(in crate::gui_app) fn random_audition_span_for_unit(
    duration_seconds: f32,
    unit: f32,
) -> (f32, f32) {
    if duration_seconds <= RANDOM_AUDITION_SECONDS {
        return (0.0, 1.0);
    }

    let width = (RANDOM_AUDITION_SECONDS / duration_seconds).clamp(0.0, 1.0);
    let max_start = 1.0 - width;
    let start = unit.clamp(0.0, 1.0) * max_start;
    (start, start + width)
}

fn log_slow_playback_phase(
    event: &'static str,
    file_name: &str,
    source_kind: &'static str,
    started_at: Instant,
) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(4) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::playback",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        file_name,
        source_kind,
        "Slow playback UI phase"
    );
}
