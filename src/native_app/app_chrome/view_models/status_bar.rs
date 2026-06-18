use crate::native_app::app::{FolderScanProgress, NativeAppState, NormalizationProgress};
use radiant::prelude as ui;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct StatusBarViewModel {
    pub(in crate::native_app) selected_sample_count: usize,
    pub(in crate::native_app) status_text: String,
    pub(in crate::native_app) worker_progress: Option<WorkerProgressViewModel>,
    pub(in crate::native_app) progress_tick: f32,
}

impl StatusBarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        Self {
            selected_sample_count: state.library.folder_browser.selected_audio_file_count(),
            status_text: bottom_status_text(state),
            worker_progress: active_worker_progress(state),
            progress_tick: state.background.progress_tick,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct WorkerProgressViewModel {
    pub(in crate::native_app) completed: usize,
    pub(in crate::native_app) total: usize,
    pub(in crate::native_app) current_fraction: Option<f32>,
    pub(in crate::native_app) active_animation: bool,
}

fn bottom_status_text(state: &NativeAppState) -> String {
    if let Some(progress) = state.library.folder_progress() {
        let counters = ui::ProgressSnapshot::new(progress.completed, progress.total);
        return if counters.is_indeterminate() {
            format!(
                "{} {} | {}",
                progress.phase,
                progress.label,
                counters.count_label("items found")
            )
        } else {
            format!(
                "{} {} | {} | {}",
                progress.phase,
                progress.label,
                counters.count_label("items found"),
                progress.detail
            )
        };
    }
    if let Some(progress) = state.background.normalization_progress.as_ref() {
        let counters = ui::ProgressSnapshot::new(progress.completed, progress.total);
        let queue = normalization_queue_status(progress.queued);
        return if counters.is_indeterminate() {
            format!(
                "Normalizing {} | {}{}",
                progress.label, progress.detail, queue
            )
        } else {
            format!(
                "Normalizing {} | {} | {}{}",
                progress.label,
                counters.count_label("items found"),
                progress.detail,
                queue
            )
        };
    }
    if let Some(progress) = WorkerProgressViewModel::from_source_cache_warm(state) {
        return source_cache_warm_status_text(state, progress);
    }
    state.ui.status.sample.clone()
}

fn normalization_queue_status(queued: usize) -> String {
    if queued == 0 {
        String::new()
    } else {
        format!(" | {queued} queued")
    }
}

fn active_worker_progress(state: &NativeAppState) -> Option<WorkerProgressViewModel> {
    state
        .library
        .folder_progress()
        .map(WorkerProgressViewModel::from_folder_progress)
        .or_else(|| {
            state
                .background
                .normalization_progress
                .as_ref()
                .map(WorkerProgressViewModel::from_normalization_progress)
        })
        .or_else(|| WorkerProgressViewModel::from_source_cache_warm(state))
}

impl WorkerProgressViewModel {
    fn from_folder_progress(progress: &FolderScanProgress) -> Self {
        Self {
            completed: progress.completed,
            total: progress.total,
            current_fraction: None,
            active_animation: false,
        }
    }

    fn from_normalization_progress(progress: &NormalizationProgress) -> Self {
        Self {
            completed: progress.work_completed,
            total: progress.work_total,
            current_fraction: None,
            active_animation: false,
        }
    }

    fn from_source_cache_warm(state: &NativeAppState) -> Option<Self> {
        let cache = &state.waveform.cache;
        (cache.active_folder_warm_folder_id.is_some() && cache.active_folder_warm_total > 0)
            .then_some(Self {
                completed: cache.active_folder_warm_completed,
                total: cache.active_folder_warm_total,
                current_fraction: cache
                    .active_folder_warm_current
                    .as_ref()
                    .map(|_| cache.active_folder_warm_current_progress.clamp(0.0, 1.0)),
                active_animation: true,
            })
    }
}

fn source_cache_warm_status_text(
    state: &NativeAppState,
    progress: WorkerProgressViewModel,
) -> String {
    let counters = ui::ProgressSnapshot::new(progress.completed, progress.total);
    let detail = state
        .waveform
        .cache
        .active_folder_warm_current
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_string());
    let stage = state
        .waveform
        .cache
        .active_folder_warm_current_stage
        .map(|stage| {
            let percent = (state
                .waveform
                .cache
                .active_folder_warm_current_progress
                .clamp(0.0, 1.0)
                * 100.0)
                .round() as usize;
            format!("{} {percent}%", stage.label())
        });
    match (stage, detail) {
        (Some(stage), Some(detail)) => format!(
            "Caching source samples | {} | {} | {}",
            counters.count_label("cached"),
            stage,
            detail
        ),
        (None, Some(detail)) => format!(
            "Caching source samples | {} | {}",
            counters.count_label("cached"),
            detail
        ),
        (Some(stage), None) => format!(
            "Caching source samples | {} | {}",
            counters.count_label("cached"),
            stage
        ),
        (None, None) => format!(
            "Caching source samples | {}",
            counters.count_label("cached")
        ),
    }
}
