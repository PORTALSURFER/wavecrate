use rand::Rng;
use span::ResolvedPlaybackSpan;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use wavecrate::audio::{AudioPlayer, edit_fade_range_from_selection};

pub(in crate::native_app) const PLAYBACK_START_ACTIVE_SOURCE_GRACE: Duration =
    Duration::from_millis(120);

use crate::native_app::app::{
    GuiMessage, NativeAppState, PendingPlaybackStart, PendingSamplePlayback, emit_gui_action,
    sample_path_label,
};
use radiant::prelude as ui;

mod loop_control;
mod progress;
mod span;

const RANDOM_AUDITION_SECONDS: f32 = 4.0;

impl NativeAppState {
    pub(in crate::native_app) fn random_playback_available(&self) -> bool {
        self.waveform.has_loaded_sample()
            || self.folder_browser.selected_file_id().is_some()
            || self.folder_browser.random_playback_available()
    }

    pub(in crate::native_app) fn play_selected_sample(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
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

    pub(in crate::native_app) fn play_waveform_from_ratio(&mut self, start_ratio: f32) {
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

    pub(in crate::native_app) fn play_random_sample_range(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let mut rng = rand::rng();
        self.play_random_sample_range_with_unit(rng.random::<f32>(), context);
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(in crate::native_app) fn play_random_sample_range_with_unit(
        &mut self,
        unit: f32,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if let Some(path) = self.folder_browser.selected_file_id().map(str::to_owned)
            && self.waveform.path() != Path::new(&path)
        {
            let label = sample_path_label(&path);
            emit_gui_action(
                "playback.play_random_sample_range",
                Some("transport"),
                Some(&label),
                "load_queued",
                started_at,
                None,
            );
            self.pending_sample_playback = Some(PendingSamplePlayback::RandomAudition { unit });
            self.load_sample_without_autoplay(path, context);
            return;
        }

        if !self.waveform.has_loaded_sample()
            && let Some(path) = self.folder_browser.random_playback_candidate(unit)
        {
            let label = sample_path_label(&path);
            emit_gui_action(
                "playback.play_random_sample_range",
                Some("transport"),
                Some(&label),
                "load_queued",
                started_at,
                None,
            );
            self.pending_sample_playback = Some(PendingSamplePlayback::RandomAudition { unit });
            self.folder_browser
                .focus_file_across_sources(Path::new(&path));
            self.load_sample_without_autoplay(path, context);
            return;
        }
        let file_name = self.waveform.file_name();
        let span = self.random_audition_span_for_loaded_waveform(unit);
        let was_looping = self.loop_playback;
        self.loop_playback = false;

        match self.start_playback_current_span(span.start, span.end) {
            Ok(()) => {
                self.sample_status = span.status_message(&file_name);
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
                if self.pending_playback_start.is_none() {
                    self.loop_playback = was_looping;
                }
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

    pub(in crate::native_app) fn random_audition_span_for_loaded_waveform(
        &mut self,
        unit: f32,
    ) -> RandomAuditionSpan {
        if let Some(range) = self
            .waveform
            .select_marked_play_range_for_random_audition(unit)
        {
            return RandomAuditionSpan {
                start: range.start(),
                end: range.end(),
                source: RandomAuditionSource::MarkedRange,
            };
        }
        let (start, end) = random_audition_span_for_unit(self.waveform.duration_seconds(), unit);
        RandomAuditionSpan {
            start,
            end,
            source: RandomAuditionSource::FixedWindow,
        }
    }

    pub(in crate::native_app) fn stop_playback(&mut self) {
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

    pub(in crate::native_app) fn start_playback_current_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_span(start_ratio, end_ratio, None)
    }

    pub(in crate::native_app) fn start_playback_span(
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
        let playback_cache_file = self.waveform.playback_cache_file();
        let source_kind = if self.waveform.playback_samples().is_some() {
            "decoded_samples"
        } else if playback_cache_file.is_some() {
            "interleaved_f32_file"
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
        } else if let Some(cache_file) = playback_cache_file {
            player.set_interleaved_f32_file_with_metadata(
                cache_file.path,
                cache_file.sample_count,
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
        player.set_edit_fade_state(edit_fade_range_from_selection(
            self.waveform.edit_selection(),
        ));
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

    pub(in crate::native_app) fn resolve_playback_span(
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum RandomAuditionSource {
    FixedWindow,
    MarkedRange,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct RandomAuditionSpan {
    pub(in crate::native_app) start: f32,
    pub(in crate::native_app) end: f32,
    pub(in crate::native_app) source: RandomAuditionSource,
}

impl RandomAuditionSpan {
    pub(in crate::native_app) fn status_message(self, file_name: &str) -> String {
        match self.source {
            RandomAuditionSource::FixedWindow => {
                format!(
                    "Random audition {file_name} from {:.1}%",
                    self.start * 100.0
                )
            }
            RandomAuditionSource::MarkedRange => {
                format!(
                    "Random marked range {file_name} from {:.1}%",
                    self.start * 100.0
                )
            }
        }
    }
}

pub(in crate::native_app) fn random_audition_span_for_unit(
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
