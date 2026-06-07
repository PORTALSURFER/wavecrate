use std::{path::PathBuf, sync::mpsc::Sender, time::Instant};

use radiant::prelude as ui;
use wavecrate::audio::AudioPlayer;

use crate::native_app::app::{
    GuiMessage, NativeAppState, NormalizationProgress, NormalizationResult,
    NormalizedWaveformReload, WaveformPlaybackResume, emit_gui_action, sample_path_label,
};
use crate::native_app::browser::file_actions::normalize_wav_file_in_place;

impl NativeAppState {
    pub(in crate::native_app) fn normalize_selected_samples(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.normalization_progress.is_some() {
            self.sample_status = String::from("Normalization already in progress");
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "busy",
                started_at,
                None,
            );
            return;
        }
        let paths = self.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.sample_status = String::from("Select a sample to normalize");
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        }

        let request = self.prepare_normalization_request(paths);
        let label = normalize_progress_label(request.paths.len());
        self.normalization_progress = Some(NormalizationProgress {
            task_id: request.task_id,
            label: label.clone(),
            completed: 0,
            total: request.paths.len(),
            detail: String::from("Queued"),
        });
        self.sample_status = format!("Normalizing {label}");
        context.spawn(
            "gui-normalize-selected-samples",
            move || run_normalization_worker(request),
            GuiMessage::NormalizationFinished,
        );
        emit_gui_action(
            "browser.normalize_selected_samples",
            Some("browser"),
            None,
            "queued",
            started_at,
            None,
        );
    }

    fn prepare_normalization_request(&mut self, paths: Vec<PathBuf>) -> NormalizationWorkerRequest {
        let loaded_path = self.waveform.path();
        let normalizing_loaded = paths.iter().any(|path| path == &loaded_path);
        let was_playing = self.waveform.is_playing() && normalizing_loaded;
        let restart_ratio = self
            .audio_player
            .as_ref()
            .and_then(AudioPlayer::progress)
            .or(self.waveform.playhead_ratio())
            .unwrap_or(0.0);
        let restart_span = self.current_playback_span;
        if was_playing {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }

        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        NormalizationWorkerRequest {
            task_id,
            paths,
            loaded_path,
            normalizing_loaded,
            was_playing,
            restart_ratio,
            restart_span,
            sender: self.worker_sender.clone(),
        }
    }

    pub(in crate::native_app) fn apply_normalization_progress(
        &mut self,
        progress: NormalizationProgress,
    ) {
        if self
            .normalization_progress
            .as_ref()
            .is_some_and(|active| active.task_id == progress.task_id)
        {
            self.normalization_progress = Some(progress);
        }
    }

    pub(in crate::native_app) fn finish_normalization(&mut self, result: NormalizationResult) {
        let started_at = Instant::now();
        if self
            .normalization_progress
            .as_ref()
            .is_none_or(|active| active.task_id != result.task_id)
        {
            return;
        }
        self.normalization_progress = None;
        self.progress_tick = 0.0;

        for path in &result.normalized {
            self.folder_browser.refresh_file_path(path);
        }

        let mut last_error = result.last_error;
        if result.normalizing_loaded
            && result
                .normalized
                .iter()
                .any(|path| path == &result.loaded_path)
        {
            let playback = result.was_playing.then_some(WaveformPlaybackResume {
                start_ratio: result.restart_ratio,
                span: result.restart_span,
            });
            if let Err(error) = self.reload_normalized_waveform(NormalizedWaveformReload {
                path: &result.loaded_path,
                playback,
            }) {
                last_error = Some(error);
            }
        }

        self.finish_normalization_status(result.normalized, last_error, started_at);
    }

    fn finish_normalization_status(
        &mut self,
        normalized: Vec<PathBuf>,
        last_error: Option<String>,
        started_at: Instant,
    ) {
        if let Some(error) = last_error {
            self.sample_status = format!(
                "Normalized {} sample{} | {error}",
                normalized.len(),
                if normalized.len() == 1 { "" } else { "s" }
            );
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "partial_or_error",
                started_at,
                Some(&error),
            );
            return;
        }

        self.sample_status = match normalized.as_slice() {
            [] => String::from("No selected samples were normalized"),
            [path] => format!("Normalized {}", sample_path_label(path)),
            _ => format!("Normalized {} samples", normalized.len()),
        };
        emit_gui_action(
            "browser.normalize_selected_samples",
            Some("browser"),
            None,
            "success",
            started_at,
            None,
        );
    }
}

struct NormalizationWorkerRequest {
    task_id: u64,
    paths: Vec<PathBuf>,
    loaded_path: PathBuf,
    normalizing_loaded: bool,
    was_playing: bool,
    restart_ratio: f32,
    restart_span: Option<(f32, f32)>,
    sender: Sender<GuiMessage>,
}

fn normalize_progress_label(count: usize) -> String {
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
}

fn run_normalization_worker(request: NormalizationWorkerRequest) -> NormalizationResult {
    let total = request.paths.len();
    let label = normalize_progress_label(total);
    let mut normalized = Vec::new();
    let mut last_error = None;
    for (index, path) in request.paths.iter().enumerate() {
        let detail = sample_path_label(path);
        send_normalization_progress(&request, label.as_str(), index, total, detail.clone());
        match normalize_wav_file_in_place(path) {
            Ok(()) => normalized.push(path.clone()),
            Err(error) => {
                last_error = Some(format!("{detail}: {error}"));
            }
        }
        send_normalization_progress(&request, label.as_str(), index + 1, total, detail);
    }
    NormalizationResult {
        task_id: request.task_id,
        loaded_path: request.loaded_path,
        normalizing_loaded: request.normalizing_loaded,
        was_playing: request.was_playing,
        restart_ratio: request.restart_ratio,
        restart_span: request.restart_span,
        normalized,
        last_error,
    }
}

fn send_normalization_progress(
    request: &NormalizationWorkerRequest,
    label: &str,
    completed: usize,
    total: usize,
    detail: String,
) {
    let _ = request
        .sender
        .send(GuiMessage::NormalizationProgress(NormalizationProgress {
            task_id: request.task_id,
            label: label.to_string(),
            completed,
            total,
            detail,
        }));
}
