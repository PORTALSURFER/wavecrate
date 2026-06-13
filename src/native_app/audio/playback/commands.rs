use radiant::prelude as ui;
use rand::Rng;
use std::{path::Path, time::Instant};

use super::random_audition::{
    RandomAuditionSource, RandomAuditionSpan, random_audition_span_for_unit,
};
use crate::native_app::app::{
    GuiMessage, NativeAppState, PendingSamplePlayback, emit_gui_action, sample_path_label,
};

impl NativeAppState {
    pub(in crate::native_app) fn random_playback_available(&self) -> bool {
        self.waveform.current.has_loaded_sample()
            || self.library.folder_browser.selected_file_id().is_some()
            || self.library.folder_browser.random_playback_available()
    }

    pub(in crate::native_app) fn play_selected_sample(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if let Some(path) = self.library.folder_browser.selected_file_id()
            && self.waveform.current.path() != Path::new(path)
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
            .current
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        match self.start_playback_current_span(start, end) {
            Ok(()) => {
                let file_name = self.waveform.current.file_name();
                self.ui.status.sample = format!("Playing {file_name}");
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
                self.ui.status.sample = format!("Playback unavailable: {err}");
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
                let file_name = self.waveform.current.file_name();
                self.ui.status.sample =
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
                self.ui.status.sample = format!("Playback unavailable: {err}");
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
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let mut rng = rand::rng();
        self.play_random_sample_range_with_unit(rng.random::<f32>(), context);
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(in crate::native_app) fn play_random_sample_range_with_unit(
        &mut self,
        unit: f32,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if let Some(path) = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned)
            && self.waveform.current.path() != Path::new(&path)
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
            self.audio.pending_sample_playback =
                Some(PendingSamplePlayback::RandomAudition { unit });
            self.load_sample_without_autoplay(path, context);
            return;
        }

        if !self.waveform.current.has_loaded_sample()
            && let Some(path) = self.library.folder_browser.random_playback_candidate(unit)
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
            self.audio.pending_sample_playback =
                Some(PendingSamplePlayback::RandomAudition { unit });
            self.library
                .folder_browser
                .focus_file_across_sources(Path::new(&path));
            self.load_sample_without_autoplay(path, context);
            return;
        }
        let file_name = self.waveform.current.file_name();
        let span = self.random_audition_span_for_loaded_waveform(unit);
        let was_looping = self.audio.loop_playback;
        self.audio.loop_playback = false;

        match self.start_playback_current_span(span.start, span.end) {
            Ok(()) => {
                self.ui.status.sample = span.status_message(&file_name);
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
                if self.audio.pending_playback_start.is_none() {
                    self.audio.loop_playback = was_looping;
                }
                self.ui.status.sample = format!("Playback unavailable: {err}");
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
            .current
            .select_marked_play_range_for_random_audition(unit)
        {
            return RandomAuditionSpan {
                start: range.start(),
                end: range.end(),
                source: RandomAuditionSource::MarkedRange,
            };
        }
        let (start, end) =
            random_audition_span_for_unit(self.waveform.current.duration_seconds(), unit);
        RandomAuditionSpan {
            start,
            end,
            source: RandomAuditionSource::FixedWindow,
        }
    }

    pub(in crate::native_app) fn stop_playback(&mut self) {
        let started_at = Instant::now();
        if let Some(player) = self.audio.player.as_mut() {
            player.stop();
        }
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        let file_name = self.waveform.current.file_name();
        self.ui.status.sample = format!("Stopped {file_name}");
        emit_gui_action(
            "playback.stop",
            Some("transport"),
            Some(&file_name),
            "success",
            started_at,
            None,
        );
    }
}
