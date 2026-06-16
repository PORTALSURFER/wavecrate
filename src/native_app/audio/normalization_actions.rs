use std::{path::PathBuf, sync::mpsc::Sender, time::Instant};

use radiant::prelude as ui;

use crate::native_app::app::{
    GuiMessage, NativeAppState, NormalizationProgress, NormalizationQueueItem, NormalizationResult,
    NormalizedWaveformReload, WaveformPlaybackResume, emit_gui_action, sample_path_label,
};
use crate::native_app::sample_library::file_actions::{
    WavNormalizationOutcome, normalize_wav_file_in_place,
};

impl NativeAppState {
    pub(in crate::native_app) fn normalize_selected_samples(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let paths = self.library.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.ui.status.sample = String::from("Select a sample to normalize");
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

        self.pause_active_folder_cache_warm(context);
        if self.background.normalization_progress.is_some() {
            self.enqueue_normalization_paths(paths, started_at);
            return;
        }
        self.start_normalization_paths(paths, context, started_at);
    }

    fn enqueue_normalization_paths(&mut self, paths: Vec<PathBuf>, started_at: Instant) {
        self.background
            .normalization_queue
            .push_back(NormalizationQueueItem { paths });
        let queued = self.background.normalization_queue.len();
        if let Some(progress) = self.background.normalization_progress.as_mut() {
            progress.queued = queued;
        }
        self.ui.status.sample = format!(
            "Queued normalization task | {queued} task{} waiting",
            if queued == 1 { "" } else { "s" }
        );
        emit_gui_action(
            "browser.normalize_selected_samples",
            Some("browser"),
            None,
            "queued_pending",
            started_at,
            None,
        );
    }

    fn start_normalization_paths(
        &mut self,
        paths: Vec<PathBuf>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        self.pause_active_folder_cache_warm(context);
        let request = self.prepare_normalization_request(paths);
        let label = normalize_progress_label(request.paths.len());
        let queued = self.background.normalization_queue.len();
        self.background.normalization_progress = Some(NormalizationProgress {
            task_id: request.task_id,
            label: label.clone(),
            completed: 0,
            total: request.paths.len(),
            queued,
            detail: String::from("Queued"),
        });
        self.ui.status.sample = format!("Normalizing {label}");
        context
            .business()
            .background("gui-normalize-selected-samples")
            .run(
                move |_| run_normalization_worker(request),
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
        let loaded_path = self.waveform.current.path();
        let normalizing_loaded = paths.iter().any(|path| path == &loaded_path);
        let was_playing = self.waveform.current.is_playing() && normalizing_loaded;
        let restart_ratio = self
            .audio
            .playback_progress
            .progress
            .or(self.waveform.current.playhead_ratio())
            .unwrap_or(0.0);
        let restart_span = self.audio.current_playback_span;
        if was_playing {
            self.stop_audio_output_playback();
            self.waveform.current.stop_playback();
            self.audio.current_playback_span = None;
        }

        let task_id = self.background.next_task_id();
        NormalizationWorkerRequest {
            task_id,
            paths,
            loaded_path,
            normalizing_loaded,
            was_playing,
            restart_ratio,
            restart_span,
            sender: self.background.worker_sender.clone(),
        }
    }

    pub(in crate::native_app) fn apply_normalization_progress(
        &mut self,
        mut progress: NormalizationProgress,
    ) {
        if self
            .background
            .normalization_progress
            .as_ref()
            .is_some_and(|active| active.task_id == progress.task_id)
        {
            progress.queued = self.background.normalization_queue.len();
            self.background.normalization_progress = Some(progress);
        }
    }

    pub(in crate::native_app) fn finish_normalization(
        &mut self,
        result: NormalizationResult,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self
            .background
            .normalization_progress
            .as_ref()
            .is_none_or(|active| active.task_id != result.task_id)
        {
            return;
        }
        self.background.normalization_progress = None;
        self.background.progress_tick = 0.0;

        for path in &result.normalized {
            self.evict_waveform_cache_path(path);
            self.library.folder_browser.refresh_file_path(path);
        }

        let last_error = result.last_error;
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
            self.reload_normalized_waveform(
                NormalizedWaveformReload {
                    path: &result.loaded_path,
                    playback,
                },
                context,
            );
        }

        self.finish_normalization_status(result.normalized, result.skipped, last_error, started_at);
        self.start_next_queued_normalization(context);
    }

    fn start_next_queued_normalization(&mut self, context: &mut ui::UiUpdateContext<GuiMessage>) {
        let Some(next) = self.background.normalization_queue.pop_front() else {
            return;
        };
        self.start_normalization_paths(next.paths, context, Instant::now());
    }

    fn finish_normalization_status(
        &mut self,
        normalized: Vec<PathBuf>,
        skipped: Vec<PathBuf>,
        last_error: Option<String>,
        started_at: Instant,
    ) {
        if let Some(error) = last_error {
            self.ui.status.sample = format!(
                "Normalized {} sample{} | skipped {} | {error}",
                normalized.len(),
                if normalized.len() == 1 { "" } else { "s" },
                skipped.len()
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

        self.ui.status.sample = match (normalized.as_slice(), skipped.as_slice()) {
            ([], []) => String::from("No selected samples were normalized"),
            ([], [path]) => format!("Already normalized {}", sample_path_label(path)),
            ([], skipped) => format!("Already normalized {} samples", skipped.len()),
            ([path], []) => format!("Normalized {}", sample_path_label(path)),
            (_, []) => format!("Normalized {} samples", normalized.len()),
            ([path], skipped) => format!(
                "Normalized {} | skipped {} sample{}",
                sample_path_label(path),
                skipped.len(),
                if skipped.len() == 1 { "" } else { "s" }
            ),
            (normalized, skipped) => format!(
                "Normalized {} samples | skipped {}",
                normalized.len(),
                skipped.len()
            ),
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
    let mut skipped = Vec::new();
    let mut last_error = None;
    for (index, path) in request.paths.iter().enumerate() {
        let detail = sample_path_label(path);
        send_normalization_progress(&request, label.as_str(), index, total, detail.clone());
        match normalize_wav_file_in_place(path) {
            Ok(WavNormalizationOutcome::Normalized) => normalized.push(path.clone()),
            Ok(WavNormalizationOutcome::Skipped) => skipped.push(path.clone()),
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
        skipped,
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
            queued: 0,
            detail,
        }));
}
