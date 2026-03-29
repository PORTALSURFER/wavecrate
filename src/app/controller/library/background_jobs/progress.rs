use super::*;
use crate::app::state::ProgressTaskKind;

/// Ensure the progress panel is visible and configured for the provided task.
pub(crate) fn ensure_progress_visible(
    controller: &mut AppController,
    task: ProgressTaskKind,
    label: &str,
    total: usize,
    cancellable: bool,
) {
    if !controller.ui.progress.visible || controller.ui.progress.task != Some(task) {
        controller.show_status_progress(task, label, total, cancellable);
        return;
    }
    controller.update_status_progress_title(task, label);
    controller.ui.progress.cancelable = cancellable;
}

/// Update detail text for an active progress task without changing total counts.
pub(crate) fn update_progress_detail(
    controller: &mut AppController,
    task: ProgressTaskKind,
    completed: usize,
    detail: Option<String>,
) {
    if controller.ui.progress.task == Some(task) {
        controller
            .ui
            .progress
            .set_counts(controller.ui.progress.total, completed);
        controller.ui.progress.set_detail(detail);
    }
}

/// Update totals/completed counts and optional detail text for an active progress task.
pub(crate) fn update_progress_totals(
    controller: &mut AppController,
    task: ProgressTaskKind,
    total: usize,
    completed: usize,
    detail: Option<String>,
) {
    if controller.ui.progress.task == Some(task) {
        controller.ui.progress.set_counts(total, completed);
        controller.ui.progress.set_detail(detail);
    }
}
