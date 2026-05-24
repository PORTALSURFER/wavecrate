use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use super::{
    GuiAppState, GuiMessage, SampleLoadResult, WaveformState, emit_gui_action, sample_path_label,
};

const SAMPLE_LOAD_PROGRESS_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const SAMPLE_LOAD_PROGRESS_MIN_DELTA: f32 = 0.01;

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
                let progress_reporter =
                    std::cell::RefCell::new(SampleLoadProgressReporter::new(sender, ticket));
                let result =
                    WaveformState::load_path_with_progress(PathBuf::from(&path), |progress| {
                        progress_reporter.borrow_mut().report(progress);
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

struct SampleLoadProgressReporter {
    sender: std::sync::mpsc::Sender<GuiMessage>,
    ticket: ui::TaskTicket,
    last_sent_at: Option<Instant>,
    last_progress: f32,
}

impl SampleLoadProgressReporter {
    fn new(sender: std::sync::mpsc::Sender<GuiMessage>, ticket: ui::TaskTicket) -> Self {
        Self {
            sender,
            ticket,
            last_sent_at: None,
            last_progress: 0.0,
        }
    }

    fn report(&mut self, progress: f32) {
        self.report_at(progress, Instant::now());
    }

    fn report_at(&mut self, progress: f32, now: Instant) {
        let progress = progress.clamp(0.0, 0.995);
        if !self.should_send(progress, now) {
            return;
        }
        self.last_sent_at = Some(now);
        self.last_progress = progress;
        let _ = self
            .sender
            .send(GuiMessage::SampleLoadProgress(self.ticket, progress));
    }

    fn should_send(&self, progress: f32, now: Instant) -> bool {
        if progress >= 0.995 {
            return true;
        }
        let Some(last_sent_at) = self.last_sent_at else {
            return true;
        };
        if progress <= self.last_progress {
            return false;
        }
        now.duration_since(last_sent_at) >= SAMPLE_LOAD_PROGRESS_MIN_INTERVAL
            && progress - self.last_progress >= SAMPLE_LOAD_PROGRESS_MIN_DELTA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn sample_load_progress_reporter_coalesces_tight_progress_loop() {
        let (sender, receiver) = mpsc::channel();
        let ticket = ui::LatestTask::new().begin();
        let mut reporter = SampleLoadProgressReporter::new(sender, ticket);
        let start = Instant::now();

        reporter.report_at(0.001, start);
        reporter.report_at(0.002, start + Duration::from_millis(1));
        reporter.report_at(0.003, start + Duration::from_millis(2));
        reporter.report_at(0.012, start + Duration::from_millis(3));
        reporter.report_at(0.014, start + Duration::from_millis(60));

        let messages = receiver.try_iter().collect::<Vec<_>>();
        assert_eq!(
            messages.len(),
            2,
            "tight progress callbacks should be coalesced so drag hover events are not starved"
        );
        assert!(matches!(
            messages.last(),
            Some(GuiMessage::SampleLoadProgress(_, progress)) if (*progress - 0.014).abs() < f32::EPSILON
        ));
    }
}
