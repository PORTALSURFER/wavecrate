use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::palette::{ACCENT, ACCENT_SOFT};
#[cfg(test)]
use crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel;
use crate::native_app::ui::ids::WORKER_PROGRESS_ROOT_ID;
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
        WorkerProgressBarContentProjection::Activity => source_processing_activity_track()
            .id(WORKER_PROGRESS_ROOT_ID)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(WORKER_PROGRESS_HEIGHT),
        WorkerProgressBarContentProjection::OverallWithActivity { progress } => {
            overall_progress_bar(progress, projection.progress_tick)
                .id(WORKER_PROGRESS_ROOT_ID)
                .width(WORKER_PROGRESS_TRACK_WIDTH)
                .height(WORKER_PROGRESS_HEIGHT)
        }
        WorkerProgressBarContentProjection::Overall { progress } => {
            overall_progress_bar(progress, projection.progress_tick)
                .id(WORKER_PROGRESS_ROOT_ID)
                .width(WORKER_PROGRESS_TRACK_WIDTH)
                .height(WORKER_PROGRESS_HEIGHT)
        }
        WorkerProgressBarContentProjection::Layered {
            overall,
            current_fraction,
        } => layered_worker_progress(overall, current_fraction, projection.progress_tick),
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

fn layered_worker_progress(
    overall: ui::ProgressSnapshot,
    current_fraction: Option<f32>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    ui::column([
        overall_progress_bar(overall, progress_tick)
            .key(identity::WORKER_PROGRESS_OVERALL_KEY)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(OVERALL_PROGRESS_HEIGHT),
        active_worker_activity_bar(current_fraction, progress_tick)
            .key(identity::WORKER_PROGRESS_ACTIVE_KEY)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(ACTIVE_PROGRESS_HEIGHT),
    ])
    .spacing(1.0)
    .id(WORKER_PROGRESS_ROOT_ID)
    .width(WORKER_PROGRESS_TRACK_WIDTH)
    .height(SOURCE_CACHE_PROGRESS_HEIGHT)
}

fn overall_progress_bar(
    progress: ui::ProgressSnapshot,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    ui::progress_bar_for_snapshot(progress, progress_tick)
        .colors(ui::Rgba8::new(48, 50, 51, 210), ACCENT.with_alpha(210))
        .max_track_height(OVERALL_TRACK_HEIGHT)
        .activatable()
        .message(GuiMessage::ToggleJobDetails)
}

fn source_processing_activity_track() -> ui::View<GuiMessage> {
    ui::determinate_progress_bar(0.0)
        .colors(ui::Rgba8::new(48, 50, 51, 210), ACCENT.with_alpha(210))
        .max_track_height(OVERALL_TRACK_HEIGHT)
        .activatable()
        .message(GuiMessage::ToggleJobDetails)
}

fn active_worker_activity_bar(
    current_fraction: Option<f32>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    let activity = ui::indeterminate_progress_bar(progress_tick)
        .colors(ui::Rgba8::new(0, 0, 0, 0), ACCENT_SOFT.with_alpha(210))
        .max_track_height(ACTIVITY_HIGHLIGHT_HEIGHT)
        .activatable()
        .message(GuiMessage::ToggleJobDetails)
        .key(identity::WORKER_PROGRESS_ACTIVITY_HIGHLIGHT_KEY)
        .width(WORKER_PROGRESS_TRACK_WIDTH)
        .height(ACTIVE_PROGRESS_HEIGHT);
    let Some(current_fraction) = current_fraction else {
        return ui::indeterminate_progress_bar(progress_tick)
            .colors(ui::Rgba8::new(48, 50, 51, 190), ACCENT_SOFT.with_alpha(210))
            .max_track_height(ACTIVE_TRACK_HEIGHT)
            .activatable()
            .message(GuiMessage::ToggleJobDetails)
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(ACTIVE_PROGRESS_HEIGHT);
    };
    ui::stack([
        ui::determinate_progress_bar(current_fraction)
            .colors(ui::Rgba8::new(48, 50, 51, 190), ACCENT.with_alpha(155))
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
