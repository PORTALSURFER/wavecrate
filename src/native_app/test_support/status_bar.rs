use crate::native_app::app::{FolderScanProgress, GuiMessage, NativeAppState};
use crate::native_app::app_chrome::status_bar as chrome_status_bar;
use crate::native_app::app_chrome::view_models::status_bar::StatusBarViewModel;
use radiant::prelude as ui;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WorkerProgressProjection {
    pub(in crate::native_app) completed: usize,
    pub(in crate::native_app) total: usize,
    pub(in crate::native_app) current_fraction: Option<f32>,
    pub(in crate::native_app) active_animation: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct StatusBarProjection {
    pub(in crate::native_app) selected_sample_count: usize,
    pub(in crate::native_app) status_text: String,
    pub(in crate::native_app) worker_progress: Option<WorkerProgressProjection>,
    pub(in crate::native_app) job_details: Option<[String; 4]>,
}

pub(in crate::native_app) fn bottom_status_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    chrome_status_bar::bottom_status_bar(StatusBarViewModel::from_app_state(state))
}

pub(in crate::native_app) fn worker_progress_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    let model = StatusBarViewModel::from_app_state(state);
    chrome_status_bar::worker_progress_bar(model.worker_progress, model.progress_tick)
}

pub(in crate::native_app) fn job_details_popover(
    progress: &FolderScanProgress,
) -> ui::View<GuiMessage> {
    chrome_status_bar::job_details_popover(progress)
}

pub(in crate::native_app) fn status_bar_projection(state: &NativeAppState) -> StatusBarProjection {
    let model = StatusBarViewModel::from_app_state(state);
    StatusBarProjection {
        selected_sample_count: model.selected_sample_count,
        status_text: model.status_text,
        worker_progress: model
            .worker_progress
            .map(|progress| WorkerProgressProjection {
                completed: progress.completed,
                total: progress.total,
                current_fraction: progress.current_fraction,
                active_animation: progress.active_animation,
            }),
        job_details: model.job_details.map(|details| details.rows),
    }
}
