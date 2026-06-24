use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use radiant::prelude as ui;

use crate::native_app::app::{
    GuiMessage, NativeAppState, NormalizationFailure, NormalizationProgress,
    NormalizationQueueItem, NormalizationResult, NormalizedWaveformReload, WaveformPlaybackResume,
    emit_gui_action, sample_path_label,
};
use crate::native_app::sample_library::file_actions::{
    WavNormalizationOutcome, normalize_wav_file_in_place_with_progress,
};
use crate::native_app::sample_library::folder_browser::file_refresh::refreshed_file_entries_for_paths;

const NORMALIZATION_WORK_UNITS_PER_FILE: usize = 1_000;
const NORMALIZATION_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(80);
const NORMALIZATION_PROGRESS_MIN_UNITS: usize = 20;
const BULK_NORMALIZATION_BACKGROUND_THRESHOLD: usize = 32;
const BULK_NORMALIZATION_PROGRESS_FILE_STEP: usize = 8;
const BULK_NORMALIZATION_PACE_INTERVAL: Duration = Duration::from_millis(16);
const BULK_NORMALIZATION_PACE_SLEEP: Duration = Duration::from_millis(1);
const VERBOSE_NORMALIZATION_PROGRESS_FILE_LIMIT: usize = 64;

pub(in crate::native_app) fn normalization_priority(file_count: usize) -> ui::TaskPriority {
    if file_count > BULK_NORMALIZATION_BACKGROUND_THRESHOLD {
        ui::TaskPriority::Background
    } else {
        ui::TaskPriority::Interactive
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn normalize_selected_samples(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let paths = self.library.folder_browser.selected_normalization_paths();
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
        if let Some(error) = self.normalization_lock_error(&paths) {
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "blocked",
                started_at,
                Some(&error),
            );
            return;
        }

        self.pause_active_folder_cache_warm(context);
        if self.background.normalization_progress.is_some() {
            let paths = self.pending_normalization_paths(paths);
            if paths.is_empty() {
                self.ui.status.sample = String::from("Normalization already queued for selection");
                emit_gui_action(
                    "browser.normalize_selected_samples",
                    Some("browser"),
                    None,
                    "already_queued",
                    started_at,
                    None,
                );
                return;
            }
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
        if let Some(error) = self.normalization_lock_error(&paths) {
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "blocked",
                started_at,
                Some(&error),
            );
            return;
        }
        self.pause_active_folder_cache_warm(context);
        let source_id = self.library.folder_browser.selected_source_id().to_string();
        let Some(source_root) = self.library.folder_browser.source_root_path(&source_id) else {
            let error = String::from("Normalize failed: selected source is not available");
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "blocked",
                started_at,
                Some(&error),
            );
            return;
        };
        let request = self.prepare_normalization_request(paths, source_id, source_root);
        let label = normalize_progress_label(request.paths.len());
        let queued = self.background.normalization_queue.len();
        let priority = normalization_priority(request.paths.len());
        self.background.normalization_active_paths =
            request.paths.iter().cloned().collect::<HashSet<_>>();
        self.background.normalization_progress = Some(NormalizationProgress {
            task_id: request.task_id,
            label: label.clone(),
            completed: 0,
            total: request.paths.len(),
            work_completed: 0,
            work_total: normalization_work_total(request.paths.len()),
            queued,
            detail: String::from("Queued"),
        });
        self.ui.status.sample = if priority == ui::TaskPriority::Background {
            format!("Normalizing {label} in background")
        } else {
            format!("Normalizing {label}")
        };
        context
            .business()
            .priority("gui-normalize-selected-samples", priority)
            .stream(
                move |_context, events| run_normalization_worker(request, events),
                GuiMessage::NormalizationProgress,
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

    fn prepare_normalization_request(
        &mut self,
        paths: Vec<PathBuf>,
        source_id: String,
        source_root: PathBuf,
    ) -> NormalizationWorkerRequest {
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
            source_id,
            source_root,
            paths,
            loaded_path,
            normalizing_loaded,
            was_playing,
            restart_ratio,
            restart_span,
        }
    }

    fn normalization_lock_error(&self, paths: &[PathBuf]) -> Option<String> {
        paths.iter().find_map(|path| {
            self.library
                .folder_browser
                .file_change_lock_error(path, "Normalize")
        })
    }

    fn pending_normalization_paths(&self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut seen = self.background.normalization_active_paths.clone();
        seen.extend(
            self.background
                .normalization_queue
                .iter()
                .flat_map(|item| item.paths.iter().cloned()),
        );
        paths
            .into_iter()
            .filter(|path| seen.insert(path.clone()))
            .collect()
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
        self.background.normalization_active_paths.clear();
        self.background.progress_tick = 0.0;

        self.evict_waveform_cache_paths(&result.normalized);
        self.library
            .folder_browser
            .refresh_file_entries(&result.source_id, &result.refreshed_files);

        let normalized_loaded = result.normalizing_loaded
            && result
                .normalized
                .iter()
                .any(|path| path == &result.loaded_path);
        let skipped_loaded = result.normalizing_loaded
            && result
                .skipped
                .iter()
                .any(|path| path == &result.loaded_path);
        let failed_loaded = result.normalizing_loaded
            && result
                .failed
                .iter()
                .any(|failure| failure.path == result.loaded_path);

        if normalized_loaded {
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
        } else if result.was_playing && (skipped_loaded || failed_loaded) {
            self.resume_unchanged_normalization_playback(
                &result.loaded_path,
                result.restart_ratio,
                result.restart_span,
                context,
            );
        }

        self.finish_normalization_status(
            result.normalized,
            result.skipped,
            result.failed,
            started_at,
        );
        self.start_next_queued_normalization(context);
    }

    fn resume_unchanged_normalization_playback(
        &mut self,
        loaded_path: &Path,
        restart_ratio: f32,
        restart_span: Option<(f32, f32)>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.waveform.current.path() != loaded_path || !self.waveform.current.has_loaded_sample()
        {
            return;
        }

        let (_, previous_end) = restart_span.unwrap_or((0.0, 1.0));
        let start = restart_ratio.clamp(0.0, 1.0);
        let end = previous_end.max(start).clamp(start, 1.0);
        match self.start_playback_current_span(start, end) {
            Ok(()) => {
                self.record_selected_sample_last_played(context);
                emit_gui_action(
                    "browser.normalize_selected_samples.resume_unchanged_playback",
                    Some("browser"),
                    Some(&self.waveform.current.file_name()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                emit_gui_action(
                    "browser.normalize_selected_samples.resume_unchanged_playback",
                    Some("browser"),
                    Some(&sample_path_label(loaded_path)),
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
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
        failed: Vec<NormalizationFailure>,
        started_at: Instant,
    ) {
        if !failed.is_empty() {
            let error = normalization_failure_status(&normalized, &skipped, &failed);
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "partial_or_error",
                started_at,
                Some(error.as_str()),
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
    source_id: String,
    source_root: PathBuf,
    paths: Vec<PathBuf>,
    loaded_path: PathBuf,
    normalizing_loaded: bool,
    was_playing: bool,
    restart_ratio: f32,
    restart_span: Option<(f32, f32)>,
}

fn normalize_progress_label(count: usize) -> String {
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
}

fn run_normalization_worker(
    request: NormalizationWorkerRequest,
    events: ui::BusinessEventSink<NormalizationProgress>,
) -> NormalizationResult {
    let total = request.paths.len();
    let label = normalize_progress_label(total);
    let work_total = normalization_work_total(total);
    let mut normalized = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    let mut pacer = NormalizationWorkerPacer::new(total);
    for (index, path) in request.paths.iter().enumerate() {
        let file_started_at = Instant::now();
        let file_label = sample_path_label(path);
        let mut progress_reporter = NormalizationProgressReporter::new(
            &request,
            label.as_str(),
            index,
            total,
            work_total,
            file_label.clone(),
            events.clone(),
        );
        progress_reporter.emit(
            index,
            0.0,
            "Queued",
            force_file_start_progress(index, total),
        );
        match normalize_wav_file_in_place_with_progress(path, |fraction, phase| {
            progress_reporter.emit(index, fraction, phase, false);
            pacer.pause_if_due();
        }) {
            Ok(WavNormalizationOutcome::Normalized) => {
                normalized.push(path.clone());
                log_normalization_worker_result(path, "normalized", None, file_started_at);
            }
            Ok(WavNormalizationOutcome::Skipped) => {
                skipped.push(path.clone());
                log_normalization_worker_result(path, "skipped", None, file_started_at);
            }
            Err(error) => {
                log_normalization_worker_result(
                    path,
                    "error",
                    Some(error.as_str()),
                    file_started_at,
                );
                failed.push(NormalizationFailure {
                    path: path.clone(),
                    error,
                });
            }
        }
        progress_reporter.emit(
            index + 1,
            0.0,
            "Done",
            force_file_done_progress(index + 1, total),
        );
        pacer.pause_if_due();
    }
    emit_normalization_metadata_refresh_progress(
        &request,
        label.as_str(),
        total,
        work_total,
        &events,
    );
    let refreshed_files = refreshed_file_entries_for_paths(&normalized, &request.source_root);
    NormalizationResult {
        task_id: request.task_id,
        source_id: request.source_id,
        loaded_path: request.loaded_path,
        normalizing_loaded: request.normalizing_loaded,
        was_playing: request.was_playing,
        restart_ratio: request.restart_ratio,
        restart_span: request.restart_span,
        normalized,
        refreshed_files,
        skipped,
        failed,
    }
}

struct NormalizationWorkerPacer {
    enabled: bool,
    last_pause: Instant,
}

impl NormalizationWorkerPacer {
    fn new(total_files: usize) -> Self {
        Self {
            enabled: total_files > BULK_NORMALIZATION_BACKGROUND_THRESHOLD,
            last_pause: Instant::now(),
        }
    }

    fn pause_if_due(&mut self) {
        if !self.enabled || self.last_pause.elapsed() < BULK_NORMALIZATION_PACE_INTERVAL {
            return;
        }
        std::thread::sleep(BULK_NORMALIZATION_PACE_SLEEP);
        self.last_pause = Instant::now();
    }
}

fn force_file_start_progress(file_index: usize, total_files: usize) -> bool {
    total_files <= VERBOSE_NORMALIZATION_PROGRESS_FILE_LIMIT || file_index == 0
}

fn force_file_done_progress(completed_files: usize, total_files: usize) -> bool {
    total_files <= VERBOSE_NORMALIZATION_PROGRESS_FILE_LIMIT
        || completed_files == total_files
        || completed_files.is_multiple_of(BULK_NORMALIZATION_PROGRESS_FILE_STEP)
}

fn emit_normalization_metadata_refresh_progress(
    request: &NormalizationWorkerRequest,
    label: &str,
    total_files: usize,
    work_total: usize,
    events: &ui::BusinessEventSink<NormalizationProgress>,
) {
    let _ = events.emit(NormalizationProgress {
        task_id: request.task_id,
        label: label.to_string(),
        completed: total_files,
        total: total_files,
        work_completed: work_total,
        work_total,
        queued: 0,
        detail: String::from("Refreshing browser metadata"),
    });
}

fn normalization_failure_status(
    normalized: &[PathBuf],
    skipped: &[PathBuf],
    failed: &[NormalizationFailure],
) -> String {
    match failed {
        [failure] if normalized.is_empty() && skipped.is_empty() => format!(
            "Could not normalize {} | {}",
            sample_path_label(&failure.path),
            failure.error
        ),
        [failure] => format!(
            "Normalized {} | skipped {} | failed 1 | {}: {}",
            normalized.len(),
            skipped.len(),
            sample_path_label(&failure.path),
            failure.error
        ),
        failures => {
            let last = failures.last().expect("failed is not empty");
            format!(
                "Normalized {} | skipped {} | failed {} | last: {}: {}",
                normalized.len(),
                skipped.len(),
                failures.len(),
                sample_path_label(&last.path),
                last.error
            )
        }
    }
}

struct NormalizationProgressReporter<'a> {
    request: &'a NormalizationWorkerRequest,
    label: &'a str,
    total_files: usize,
    work_total: usize,
    file_label: String,
    events: ui::BusinessEventSink<NormalizationProgress>,
    last_emit: Instant,
    last_work_completed: usize,
    min_work_units: usize,
}

impl<'a> NormalizationProgressReporter<'a> {
    fn new(
        request: &'a NormalizationWorkerRequest,
        label: &'a str,
        file_index: usize,
        total_files: usize,
        work_total: usize,
        file_label: String,
        events: ui::BusinessEventSink<NormalizationProgress>,
    ) -> Self {
        Self {
            request,
            label,
            total_files,
            work_total,
            file_label,
            events,
            last_emit: Instant::now() - NORMALIZATION_PROGRESS_MIN_INTERVAL,
            last_work_completed: normalization_work_completed(file_index, 0.0),
            min_work_units: normalization_progress_min_units(total_files),
        }
    }

    fn emit(
        &mut self,
        completed_files: usize,
        file_fraction: f32,
        phase: &'static str,
        force: bool,
    ) {
        let work_completed =
            normalization_work_completed(completed_files, file_fraction).min(self.work_total);
        let now = Instant::now();
        let advanced = work_completed.saturating_sub(self.last_work_completed);
        if !force
            && advanced < self.min_work_units
            && now.duration_since(self.last_emit) < NORMALIZATION_PROGRESS_MIN_INTERVAL
        {
            return;
        }
        self.last_emit = now;
        self.last_work_completed = work_completed;
        let detail = if phase.is_empty() {
            self.file_label.clone()
        } else {
            format!("{} | {phase}", self.file_label)
        };
        let _ = self.events.emit(NormalizationProgress {
            task_id: self.request.task_id,
            label: self.label.to_string(),
            completed: completed_files.min(self.total_files),
            total: self.total_files,
            work_completed,
            work_total: self.work_total,
            queued: 0,
            detail,
        });
    }
}

fn normalization_progress_min_units(total_files: usize) -> usize {
    if total_files > VERBOSE_NORMALIZATION_PROGRESS_FILE_LIMIT {
        NORMALIZATION_WORK_UNITS_PER_FILE * BULK_NORMALIZATION_PROGRESS_FILE_STEP
    } else {
        NORMALIZATION_PROGRESS_MIN_UNITS
    }
}

fn normalization_work_total(total_files: usize) -> usize {
    total_files.saturating_mul(NORMALIZATION_WORK_UNITS_PER_FILE)
}

fn normalization_work_completed(completed_files: usize, file_fraction: f32) -> usize {
    let file_units =
        (file_fraction.clamp(0.0, 1.0) * NORMALIZATION_WORK_UNITS_PER_FILE as f32).round() as usize;
    completed_files
        .saturating_mul(NORMALIZATION_WORK_UNITS_PER_FILE)
        .saturating_add(file_units)
}

fn log_normalization_worker_result(
    path: &std::path::Path,
    outcome: &'static str,
    error: Option<&str>,
    started_at: Instant,
) {
    tracing::info!(
        target: "wavecrate::debug::normalization",
        event = "browser.normalize.worker.result",
        outcome,
        elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0,
        error = error.unwrap_or_default(),
        path = %path.display()
    );
}
