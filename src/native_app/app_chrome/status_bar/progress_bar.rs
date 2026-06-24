use crate::native_app::app::GuiMessage;
#[cfg(test)]
use crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel;
use radiant::prelude as ui;

use super::identity;
use super::projection::{WorkerProgressBarContentProjection, WorkerProgressBarProjection};

const WORKER_PROGRESS_TRACK_WIDTH: f32 = 180.0;
const WORKER_PROGRESS_HEIGHT: f32 = 10.0;
const SOURCE_CACHE_PROGRESS_HEIGHT: f32 = 12.0;
const OVERALL_PROGRESS_HEIGHT: f32 = 6.0;
const ACTIVE_PROGRESS_HEIGHT: f32 = 5.0;
const OVERALL_TRACK_HEIGHT: f32 = 5.0;
const ACTIVE_TRACK_HEIGHT: f32 = 4.0;
const ACTIVITY_HIGHLIGHT_HEIGHT: f32 = 2.0;

pub(super) fn worker_progress_bar_from_projection(
    projection: WorkerProgressBarProjection,
) -> ui::View<GuiMessage> {
    match projection.content {
        WorkerProgressBarContentProjection::Hidden => {
            ui::empty().width(0.0).height(WORKER_PROGRESS_HEIGHT)
        }
        WorkerProgressBarContentProjection::Overall { progress } => {
            overall_progress_bar(progress, projection.progress_tick)
                .key(identity::WORKER_PROGRESS_ROOT_KEY)
                .width(WORKER_PROGRESS_TRACK_WIDTH)
                .height(WORKER_PROGRESS_HEIGHT)
        }
        WorkerProgressBarContentProjection::SourceCache {
            overall,
            current_fraction,
        } => source_cache_worker_progress(overall, current_fraction, projection.progress_tick),
    }
}

#[cfg(test)]
pub(super) fn worker_progress_bar(
    progress: Option<WorkerProgressViewModel>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    worker_progress_bar_from_projection(WorkerProgressBarProjection::from_progress(
        progress,
        progress_tick,
    ))
}

fn source_cache_worker_progress(
    overall: ui::ProgressSnapshot,
    current_fraction: Option<f32>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    ui::column([
        overall_progress_bar(overall, progress_tick)
            .key(identity::WORKER_PROGRESS_OVERALL_KEY)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(OVERALL_PROGRESS_HEIGHT),
        active_cache_progress_bar(current_fraction, progress_tick)
            .key(identity::WORKER_PROGRESS_ACTIVE_KEY)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(ACTIVE_PROGRESS_HEIGHT),
    ])
    .spacing(1.0)
    .key(identity::WORKER_PROGRESS_ROOT_KEY)
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
        .key(identity::WORKER_PROGRESS_ACTIVITY_HIGHLIGHT_KEY)
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
            .key(identity::WORKER_PROGRESS_CURRENT_FILE_KEY)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(ACTIVE_PROGRESS_HEIGHT),
        activity,
    ])
    .width(WORKER_PROGRESS_TRACK_WIDTH)
    .height(ACTIVE_PROGRESS_HEIGHT)
}
