mod identity;
mod progress_bar;
mod projection;

use self::projection::{
    JobDetailsPopoverProjection, bottom_status_bar_projection, job_details_popover_projection,
};
use crate::native_app::app::{FolderScanProgress, GuiMessage, NativeAppState};
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::view_models::status_bar::StatusBarViewModel;
#[cfg(test)]
use crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel;
use radiant::prelude as ui;

pub(in crate::native_app) fn bottom_status_area(state: &NativeAppState) -> ui::View<GuiMessage> {
    bottom_status_bar(StatusBarViewModel::from_app_state(state))
        .overlays(status_bar_overlays(state))
}

pub(in crate::native_app) fn bottom_status_bar(model: StatusBarViewModel) -> ui::View<GuiMessage> {
    let projection = bottom_status_bar_projection(model);
    ui::status_bar_from_parts(
        ui::StatusBarParts::new(ui::StatusSegments::left_center(
            projection.selected_sample_count_label,
            projection.status_text,
        ))
        .left_width(120.0)
        .trailing(progress_bar::worker_progress_bar_from_projection(
            projection.worker_progress,
        )),
    )
}

#[cfg(test)]
pub(in crate::native_app) fn worker_progress_bar(
    progress: Option<WorkerProgressViewModel>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    progress_bar::worker_progress_bar(progress, progress_tick)
}

fn status_bar_overlays(state: &NativeAppState) -> ui::Overlays<GuiMessage> {
    ui::overlays()
        .popover_opt(job_details_overlay(state))
        .blocking_modal_opt(transaction_list_overlay(state))
}

fn job_details_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    if state.ui.chrome.job_details_open
        && let Some(progress) = state.library.folder_progress()
    {
        return Some(job_details_popover(progress));
    }

    None
}

fn transaction_list_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .ui
        .chrome
        .transaction_list_open
        .then(|| modals::transaction_list(state))
}

pub(in crate::native_app) fn job_details_popover(
    progress: &FolderScanProgress,
) -> ui::View<GuiMessage> {
    job_details_popover_from_projection(job_details_popover_projection(progress))
}

fn job_details_popover_from_projection(
    projection: JobDetailsPopoverProjection,
) -> ui::View<GuiMessage> {
    let content = ui::column(
        projection
            .rows
            .into_iter()
            .map(|row| ui::text_line(row, 20.0))
            .collect::<Vec<_>>(),
    )
    .spacing(5.0)
    .fill_width();
    ui::closeable_dialog_layer_from_parts(
        ui::DialogLayerParts::new(
            projection.title,
            content,
            ui::WidgetTone::Neutral,
            ui::Vector2::new(300.0, 132.0),
        )
        .horizontal(ui::LayerHorizontalAnchor::End)
        .vertical(ui::LayerVerticalAnchor::End)
        .inset(14.0, 38.0),
        GuiMessage::CloseJobDetails,
    )
    .key(identity::JOB_DETAILS_POPOVER_KEY)
}
