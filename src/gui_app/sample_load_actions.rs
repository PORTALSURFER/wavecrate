use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use super::{
    GuiAppState, GuiMessage, SampleLoadResult, WaveformState, emit_gui_action, sample_path_label,
};

pub(super) struct NormalizedWaveformReload<'a> {
    pub(super) path: &'a Path,
    pub(super) playback: Option<WaveformPlaybackResume>,
}

pub(super) struct WaveformPlaybackResume {
    pub(super) start_ratio: f32,
    pub(super) span: Option<(f32, f32)>,
}

impl GuiAppState {
    pub(super) fn reload_normalized_waveform(
        &mut self,
        reload: NormalizedWaveformReload<'_>,
    ) -> Result<(), String> {
        self.waveform = WaveformState::load_path(reload.path.to_path_buf())?;
        self.folder_browser
            .select_file(reload.path.display().to_string());
        if let Some(playback) = reload.playback {
            let (_, previous_end) = playback.span.unwrap_or((0.0, 1.0));
            let start = playback.start_ratio.clamp(0.0, 1.0);
            let end = previous_end.max(start).clamp(start, 1.0);
            self.start_playback_current_span(start, end)?;
        }
        Ok(())
    }

    pub(super) fn select_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.folder_browser
            .focus_file_preserving_selection(path.clone());
        self.load_sample(path, context);
    }

    pub(super) fn select_sample_with_modifiers(
        &mut self,
        path: String,
        modifiers: PointerModifiers,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.folder_browser
            .select_file_with_modifiers(path.clone(), modifiers);
        self.load_sample(path, context);
    }

    pub(super) fn load_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.waveform.is_playing() {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }
        self.sample_status = format!("Loading {}", sample_path_label(path.as_str()));
        let label = sample_path_label(path.as_str());
        self.waveform_loading_label = Some(label.clone());
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&label),
            "load_queued",
            started_at,
            None,
        );
        let ticket = self.sample_load_task.begin();
        let sender = self.worker_sender.clone();
        context.spawn(
            "gui-sample-load",
            move || {
                let result =
                    WaveformState::load_path_with_progress(PathBuf::from(&path), |progress| {
                        let _ = sender.send(GuiMessage::SampleLoadProgress(ticket, progress));
                    });
                ui::TaskCompletion {
                    ticket,
                    output: SampleLoadResult { path, result },
                }
            },
            GuiMessage::SampleLoadFinished,
        );
    }

    pub(super) fn finish_sample_load(&mut self, load: ui::TaskCompletion<SampleLoadResult>) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let load = load.output;
        let label = sample_path_label(load.path.as_str());
        if !self.sample_load_task.finish(ticket) {
            emit_gui_action(
                "browser.sample_load.finish",
                Some("browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
            return;
        }
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        match load.result {
            Ok(waveform) => {
                let file_name = waveform.file_name();
                self.waveform = waveform;
                match self.start_playback_current_span(0.0, 1.0) {
                    Ok(()) => {
                        self.sample_status = format!("Playing {file_name}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "playing",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        self.sample_status =
                            format!("Loaded {file_name} | playback unavailable: {err}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "loaded_playback_error",
                            started_at,
                            Some(&err),
                        );
                    }
                }
            }
            Err(err) => {
                self.sample_status = format!("Could not load sample: {err}");
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }
}
