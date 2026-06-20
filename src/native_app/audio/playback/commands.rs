use radiant::prelude as ui;
use rand::Rng;
use std::{path::Path, time::Instant};

use super::intent::PlaybackIntent;
use super::random_audition::{
    RandomAuditionSource, RandomAuditionSpan, RandomAuditionUnits, random_audition_span_for_units,
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
                self.record_selected_sample_last_played(context);
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

    pub(in crate::native_app) fn play_waveform_from_ratio(
        &mut self,
        start_ratio: f32,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self.start_playback_intent(PlaybackIntent::new(start_ratio, 1.0)) {
            Ok(()) => {
                let file_name = self.waveform.current.file_name();
                self.record_selected_sample_last_played(context);
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

    pub(in crate::native_app) fn play_from_current_play_start(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.waveform.current.has_loaded_sample() {
            self.play_selected_sample(context);
            return;
        }
        let start_ratio = self.waveform.current.play_mark_ratio().unwrap_or(0.0);
        self.play_waveform_from_ratio(start_ratio, context);
    }

    pub(in crate::native_app) fn play_random_sample_range(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let mut rng = rand::rng();
        let units = RandomAuditionUnits::new(rng.random::<f32>(), rng.random::<f32>());
        self.play_random_sample_range_with_units(units, context);
    }

    pub(in crate::native_app) fn play_random_sample_range_with_units(
        &mut self,
        units: RandomAuditionUnits,
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
            self.audio.pending_sample_playback = Some(PendingSamplePlayback::RandomAudition {
                start_unit: units.start,
                length_unit: units.length,
            });
            self.load_sample_without_autoplay(path, context);
            return;
        }

        if !self.waveform.current.has_loaded_sample()
            && let Some(path) = self
                .library
                .folder_browser
                .random_playback_candidate(units.start)
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
            self.audio.pending_sample_playback = Some(PendingSamplePlayback::RandomAudition {
                start_unit: units.start,
                length_unit: units.length,
            });
            self.library
                .folder_browser
                .focus_file_across_sources(Path::new(&path));
            self.load_sample_without_autoplay(path, context);
            return;
        }
        let file_name = self.waveform.current.file_name();
        let span = self.random_audition_span_for_loaded_waveform(units);

        match self.start_random_audition_span(span) {
            Ok(()) => {
                self.record_selected_sample_last_played(context);
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
        &self,
        units: RandomAuditionUnits,
    ) -> RandomAuditionSpan {
        let (start, end) =
            random_audition_span_for_units(self.waveform.current.duration_seconds(), units);
        RandomAuditionSpan {
            start,
            end,
            source: RandomAuditionSource::WholeSample,
        }
    }

    pub(in crate::native_app) fn start_random_audition_span(
        &mut self,
        span: RandomAuditionSpan,
    ) -> Result<(), String> {
        self.waveform
            .current
            .set_play_selection_range(span.start, span.end);
        self.start_playback_intent(PlaybackIntent::random_region(span.start, span.end))
    }

    pub(in crate::native_app) fn stop_playback(&mut self) {
        let started_at = Instant::now();
        self.stop_audio_output_playback();
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

    pub(in crate::native_app) fn stop_audio_output_playback(&mut self) {
        if let Some(runtime) = self.audio.playback_runtime.as_ref() {
            let _ = runtime.try_stop();
            self.audio.pending_runtime_start = None;
        } else if let Some(player) = self.audio.player.as_mut() {
            player.stop();
        }
    }
}
