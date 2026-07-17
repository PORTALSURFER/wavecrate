use crate::native_app::app::{
    FileMoveProgress, FolderScanProgress, NativeAppState, NormalizationProgress,
};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct StatusBarViewModel {
    pub(in crate::native_app) selected_sample_count: usize,
    pub(in crate::native_app) status_text: String,
    pub(in crate::native_app) status_severity: StatusSeverity,
    pub(in crate::native_app) worker_progress: Option<WorkerProgressViewModel>,
    pub(in crate::native_app) job_details: Option<JobDetailsViewModel>,
    pub(in crate::native_app) progress_tick: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum StatusSeverity {
    Normal,
    Warning,
}

impl StatusBarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        let worker = active_worker(state);
        Self {
            selected_sample_count: state.library.folder_browser.selected_audio_file_count(),
            status_text: bottom_status_text(state),
            status_severity: bottom_status_severity(state, worker.is_some()),
            worker_progress: worker.as_ref().map(|worker| worker.progress),
            job_details: worker.map(|worker| worker.details),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct JobDetailsViewModel {
    pub(in crate::native_app) rows: [String; 4],
}

struct ActiveWorkerViewModel {
    progress: WorkerProgressViewModel,
    details: JobDetailsViewModel,
}

fn bottom_status_text(state: &NativeAppState) -> String {
    let status = state.ui.status.sample.clone();
    match state.library.folder_browser.selected_source_status_label() {
        Some(source_status) if source_status.starts_with(&status) => source_status,
        Some(source_status) if !status.starts_with(&source_status) => {
            format!("{source_status} | {status}")
        }
        _ => status,
    }
}

fn bottom_status_severity(state: &NativeAppState, worker_active: bool) -> StatusSeverity {
    if !worker_active
        && state
            .library
            .folder_browser
            .source_is_missing(state.library.folder_browser.selected_source_id())
    {
        StatusSeverity::Warning
    } else {
        StatusSeverity::Normal
    }
}

fn active_worker(state: &NativeAppState) -> Option<ActiveWorkerViewModel> {
    if let Some(progress) = state.library.folder_progress() {
        return Some(ActiveWorkerViewModel {
            progress: WorkerProgressViewModel::from_folder_progress(progress),
            details: JobDetailsViewModel::from_folder_progress(progress),
        });
    }
    if let Some(progress) = state.background.normalization_progress.as_ref() {
        return Some(ActiveWorkerViewModel {
            progress: WorkerProgressViewModel::from_normalization_progress(progress),
            details: JobDetailsViewModel::from_normalization_progress(progress),
        });
    }
    if let Some(progress) = state.background.file_move_progress.as_ref() {
        return Some(ActiveWorkerViewModel {
            progress: WorkerProgressViewModel::from_file_move_progress(progress),
            details: JobDetailsViewModel::from_file_move_progress(progress),
        });
    }
    if let Some(progress) = WorkerProgressViewModel::from_source_cache_warm(state) {
        return Some(ActiveWorkerViewModel {
            progress,
            details: JobDetailsViewModel::from_source_cache_warm(state),
        });
    }
    state
        .background
        .source_processing_progress
        .as_ref()
        .map(|source_progress| ActiveWorkerViewModel {
            progress: WorkerProgressViewModel::from_source_processing(source_progress),
            details: JobDetailsViewModel::from_source_processing(state, source_progress),
        })
}

impl WorkerProgressViewModel {
    pub(in crate::native_app) fn from_folder_progress(progress: &FolderScanProgress) -> Self {
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

    fn from_file_move_progress(progress: &FileMoveProgress) -> Self {
        Self {
            completed: progress.completed,
            total: progress.total,
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

    fn from_source_processing(progress: &crate::native_app::app::SourceProcessingProgress) -> Self {
        Self {
            completed: progress.completed,
            total: progress.total,
            current_fraction: None,
            // Source processing has no independently measurable current-item fraction. Use one
            // determinate track when totals are known and let the shared progress primitive
            // animate that same track when totals are unknown. A second indeterminate strip can
            // clip down to an ambiguous edge sliver at the end of its animation cycle.
            active_animation: false,
        }
    }
}

impl JobDetailsViewModel {
    pub(in crate::native_app) fn from_folder_progress(progress: &FolderScanProgress) -> Self {
        Self {
            rows: job_rows(
                progress.phase.as_str(),
                progress.label.as_str(),
                progress.completed,
                progress.total,
                progress.detail.as_str(),
                "found",
            ),
        }
    }

    fn from_normalization_progress(progress: &NormalizationProgress) -> Self {
        let detail = if progress.queued == 0 {
            progress.detail.clone()
        } else {
            format!("{} | {} queued", progress.detail, progress.queued)
        };
        Self {
            rows: job_rows(
                "Normalization",
                progress.label.as_str(),
                progress.work_completed,
                progress.work_total,
                detail.as_str(),
                "processed",
            ),
        }
    }

    fn from_file_move_progress(progress: &FileMoveProgress) -> Self {
        Self {
            rows: job_rows(
                "File operation",
                progress.label.as_str(),
                progress.completed,
                progress.total,
                progress.detail.as_str(),
                "processed",
            ),
        }
    }

    fn from_source_cache_warm(state: &NativeAppState) -> Self {
        let cache = &state.waveform.cache;
        let checking = cache.active_folder_warm_plan_task.active().is_some();
        let path = cache
            .active_folder_warm_current
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default();
        let stage = cache.active_folder_warm_current_stage.map(|stage| {
            let percent = (cache.active_folder_warm_current_progress.clamp(0.0, 1.0) * 100.0)
                .round() as usize;
            format!("{} {percent}%", stage.label())
        });
        let detail = match (stage, path.is_empty()) {
            (Some(stage), false) => format!("{stage} | {path}"),
            (Some(stage), true) => stage,
            (None, false) => path,
            (None, true) => String::new(),
        };
        Self {
            rows: job_rows(
                if checking {
                    "Cache check"
                } else {
                    "Waveform cache"
                },
                selected_source_label(state).as_str(),
                cache.active_folder_warm_completed,
                cache.active_folder_warm_total,
                detail.as_str(),
                if checking { "checked" } else { "cached" },
            ),
        }
    }

    fn from_source_processing(
        state: &NativeAppState,
        progress: &crate::native_app::app::SourceProcessingProgress,
    ) -> Self {
        let current = if progress.detail.is_empty() {
            progress.stage.clone()
        } else {
            format!("{} | {}", progress.stage, progress.detail)
        };
        let source = state
            .library
            .folder_browser
            .source_label(progress.source_id.as_str())
            .unwrap_or(progress.source_id.as_str());
        Self {
            rows: if progress.total == 0 {
                [
                    String::from("Type: Source processing"),
                    format!("Source: {source}"),
                    String::from("Progress: Discovering work"),
                    format!("Current: {current}"),
                ]
            } else {
                job_rows(
                    "Source processing",
                    source,
                    progress.completed,
                    progress.total,
                    current.as_str(),
                    "processed",
                )
            },
        }
    }
}

fn selected_source_label(state: &NativeAppState) -> String {
    let source_id = state.library.folder_browser.selected_source_id();
    state
        .library
        .folder_browser
        .source_label(source_id)
        .unwrap_or(source_id)
        .to_string()
}

fn job_rows(
    kind: &str,
    source: &str,
    completed: usize,
    total: usize,
    detail: &str,
    indeterminate_suffix: &str,
) -> [String; 4] {
    let progress = if total == 0 {
        format!("{completed} {indeterminate_suffix}")
    } else {
        format!("{}/{}", completed.min(total), total)
    };
    let detail = if detail.is_empty() {
        "Waiting for next item"
    } else {
        detail
    };
    [
        format!("Type: {kind}"),
        format!("Source: {source}"),
        format!("Progress: {progress}"),
        format!("Current: {detail}"),
    ]
}
