mod projection;

use self::projection::{
    JobDetailsPopoverProjection, WorkerProgressBarContentProjection, WorkerProgressBarProjection,
    bottom_status_bar_projection, job_details_popover_projection,
};
use crate::native_app::app::{FolderScanProgress, GuiMessage, NativeAppState};
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::view_models::status_bar::StatusBarViewModel;
#[cfg(test)]
use crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel;
use radiant::prelude as ui;

const WORKER_PROGRESS_TRACK_WIDTH: f32 = 180.0;
const WORKER_PROGRESS_HEIGHT: f32 = 10.0;
const SOURCE_CACHE_PROGRESS_HEIGHT: f32 = 12.0;
const OVERALL_PROGRESS_HEIGHT: f32 = 6.0;
const ACTIVE_PROGRESS_HEIGHT: f32 = 5.0;
const OVERALL_TRACK_HEIGHT: f32 = 5.0;
const ACTIVE_TRACK_HEIGHT: f32 = 4.0;
const ACTIVITY_HIGHLIGHT_HEIGHT: f32 = 2.0;

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
        .trailing(worker_progress_bar_from_projection(
            projection.worker_progress,
        )),
    )
}

#[cfg(test)]
pub(in crate::native_app) fn worker_progress_bar(
    progress: Option<WorkerProgressViewModel>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    worker_progress_bar_from_projection(WorkerProgressBarProjection::from_progress(
        progress,
        progress_tick,
    ))
}

fn worker_progress_bar_from_projection(
    projection: WorkerProgressBarProjection,
) -> ui::View<GuiMessage> {
    match projection.content {
        WorkerProgressBarContentProjection::Hidden => {
            ui::empty().width(0.0).height(WORKER_PROGRESS_HEIGHT)
        }
        WorkerProgressBarContentProjection::Overall { progress } => {
            overall_progress_bar(progress, projection.progress_tick)
                .key("bottom-status-progress-bar")
                .width(WORKER_PROGRESS_TRACK_WIDTH)
                .height(WORKER_PROGRESS_HEIGHT)
        }
        WorkerProgressBarContentProjection::SourceCache {
            overall,
            current_fraction,
        } => source_cache_worker_progress(overall, current_fraction, projection.progress_tick),
    }
}

fn source_cache_worker_progress(
    overall: ui::ProgressSnapshot,
    current_fraction: Option<f32>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    ui::column([
        overall_progress_bar(overall, progress_tick)
            .key("bottom-status-progress-overall")
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(OVERALL_PROGRESS_HEIGHT),
        active_cache_progress_bar(current_fraction, progress_tick)
            .key("bottom-status-progress-active")
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(ACTIVE_PROGRESS_HEIGHT),
    ])
    .spacing(1.0)
    .key("bottom-status-progress-bar")
    .width(WORKER_PROGRESS_TRACK_WIDTH)
    .height(SOURCE_CACHE_PROGRESS_HEIGHT)
}

fn overall_progress_bar(
    progress: ui::ProgressSnapshot,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    ui::progress_bar_for_snapshot(progress, progress_tick)
        .colors(
            ui::Rgba8::new(48, 50, 51, 210),
            ui::Rgba8::new(255, 112, 86, 210),
        )
        .max_track_height(OVERALL_TRACK_HEIGHT)
        .activatable()
        .message(GuiMessage::ToggleJobDetails)
}

fn active_cache_progress_bar(
    current_fraction: Option<f32>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    let activity = ui::indeterminate_progress_bar(progress_tick)
        .colors(
            ui::Rgba8::new(0, 0, 0, 0),
            ui::Rgba8::new(255, 198, 116, 210),
        )
        .max_track_height(ACTIVITY_HIGHLIGHT_HEIGHT)
        .activatable()
        .message(GuiMessage::ToggleJobDetails)
        .key("bottom-status-progress-activity-highlight")
        .width(WORKER_PROGRESS_TRACK_WIDTH)
        .height(ACTIVE_PROGRESS_HEIGHT);
    let Some(current_fraction) = current_fraction else {
        return ui::indeterminate_progress_bar(progress_tick)
            .colors(
                ui::Rgba8::new(48, 50, 51, 190),
                ui::Rgba8::new(255, 198, 116, 210),
            )
            .max_track_height(ACTIVE_TRACK_HEIGHT)
            .activatable()
            .message(GuiMessage::ToggleJobDetails)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(ACTIVE_PROGRESS_HEIGHT);
    };
    ui::stack([
        ui::determinate_progress_bar(current_fraction)
            .colors(
                ui::Rgba8::new(48, 50, 51, 190),
                ui::Rgba8::new(255, 112, 86, 155),
            )
            .max_track_height(ACTIVE_TRACK_HEIGHT)
            .activatable()
            .message(GuiMessage::ToggleJobDetails)
            .key("bottom-status-progress-current-file")
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(ACTIVE_PROGRESS_HEIGHT),
        activity,
    ])
    .width(WORKER_PROGRESS_TRACK_WIDTH)
    .height(ACTIVE_PROGRESS_HEIGHT)
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
    .key("bottom-job-details-popover")
}
