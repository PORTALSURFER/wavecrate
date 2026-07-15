mod identity;
mod progress_bar;
mod projection;

use self::projection::{
    JobDetailsPopoverProjection, bottom_status_bar_projection, job_details_popover_projection,
};
use crate::native_app::app::{FolderScanProgress, GuiMessage, NativeAppState};
use crate::native_app::app_chrome::modals;
#[cfg(test)]
use crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel;
use crate::native_app::app_chrome::view_models::status_bar::{StatusBarViewModel, StatusSeverity};
use radiant::prelude as ui;

const STATUS_BAR_HEIGHT: f32 = 30.0;
const STATUS_BAR_SEGMENT_HEIGHT: f32 = 20.0;
const STATUS_BAR_LEFT_WIDTH: f32 = 120.0;
const STATUS_BAR_SPACING: f32 = 8.0;
const STATUS_BAR_PADDING_X: f32 = 12.0;
const STATUS_BAR_PADDING_Y: f32 = 4.0;
const SOURCE_MISSING_STATUS_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 112, 86, 230);

pub(in crate::native_app) fn bottom_status_area(state: &NativeAppState) -> ui::View<GuiMessage> {
    bottom_status_bar(StatusBarViewModel::from_app_state(state))
        .overlays(status_bar_overlays(state))
}

pub(in crate::native_app) fn bottom_status_bar(model: StatusBarViewModel) -> ui::View<GuiMessage> {
    let projection = bottom_status_bar_projection(model);
    ui::row([
        status_segment(projection.selected_sample_count_label).width(STATUS_BAR_LEFT_WIDTH),
        status_text_segment(projection.status_text, projection.status_severity).fill_width(),
        progress_bar::worker_progress_bar_from_projection(projection.worker_progress),
    ])
    .style(ui::WidgetStyle::default())
    .spacing(STATUS_BAR_SPACING)
    .padding_x(STATUS_BAR_PADDING_X)
    .padding_y(STATUS_BAR_PADDING_Y)
    .fill_width()
    .height(STATUS_BAR_HEIGHT)
}

fn status_segment(label: impl Into<ui::TextContent>) -> ui::View<GuiMessage> {
    ui::text(label).truncate().height(STATUS_BAR_SEGMENT_HEIGHT)
}

fn status_text_segment(label: String, severity: StatusSeverity) -> ui::View<GuiMessage> {
    let segment = status_segment(label);
    match severity {
        StatusSeverity::Normal => segment,
        StatusSeverity::Warning => {
            segment.text_color(ui::TextColorRole::Custom(SOURCE_MISSING_STATUS_COLOR))
        }
    }
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
    radiant::application::closeable_dialog_layer_from_parts(
        radiant::application::DialogLayerParts::new(
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

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;
    use std::sync::Arc;

    #[test]
    fn bottom_status_bar_paints_missing_source_status_as_warning() {
        let frame = bottom_status_bar(StatusBarViewModel {
            selected_sample_count: 0,
            status_text: String::from("Source missing | Ready"),
            status_severity: StatusSeverity::Warning,
            worker_progress: None,
            progress_tick: 0.0,
        })
        .view_frame_at_size_with_default_theme(ui::Vector2::new(520.0, STATUS_BAR_HEIGHT));

        assert_eq!(
            frame.paint_plan.first_text_color("Source missing | Ready"),
            Some(SOURCE_MISSING_STATUS_COLOR)
        );
    }

    #[test]
    fn status_segment_accepts_shared_text_content() {
        let label: Arc<str> = Arc::from("Shared worker status");
        let frame = status_segment(Arc::clone(&label))
            .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, STATUS_BAR_HEIGHT));
        let run = frame
            .paint_plan
            .first_text_run(label.as_ref())
            .expect("shared status label should paint");

        assert_eq!(run.text.as_str(), label.as_ref());
        assert!(!run.text.is_static());
    }
}
