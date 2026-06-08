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
            selected_sample_count: state.folder_browser.selected_audio_file_count(),
            status_text: bottom_status_text(state),
            worker_progress: active_worker_progress(state),
            progress_tick: state.progress_tick,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct WorkerProgressViewModel {
    pub(in crate::native_app) completed: usize,
    pub(in crate::native_app) total: usize,
}

fn bottom_status_text(state: &NativeAppState) -> String {
    if let Some(progress) = state.folder_progress.as_ref() {
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
    state
        .normalization_progress
        .as_ref()
        .map(|progress| {
            let counters = ui::ProgressSnapshot::new(progress.completed, progress.total);
            if counters.is_indeterminate() {
                format!("Normalizing {} | {}", progress.label, progress.detail)
            } else {
                format!(
                    "Normalizing {} | {} | {}",
                    progress.label,
                    counters.count_label("items found"),
                    progress.detail
                )
            }
        })
        .unwrap_or_else(|| state.sample_status.clone())
}

fn active_worker_progress(state: &NativeAppState) -> Option<WorkerProgressViewModel> {
    state
        .folder_progress
        .as_ref()
        .map(WorkerProgressViewModel::from_folder_progress)
        .or_else(|| {
            state
                .normalization_progress
                .as_ref()
                .map(WorkerProgressViewModel::from_normalization_progress)
        })
}

impl WorkerProgressViewModel {
    fn from_folder_progress(progress: &FolderScanProgress) -> Self {
        Self {
            completed: progress.completed,
            total: progress.total,
        }
    }

    fn from_normalization_progress(progress: &NormalizationProgress) -> Self {
        Self {
            completed: progress.completed,
            total: progress.total,
        }
    }
}
