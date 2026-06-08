use crate::native_app::app::{FolderScanProgress, GuiMessage};
use crate::native_app::app_chrome::view_models::status_bar::{
    StatusBarViewModel, WorkerProgressViewModel,
};
use radiant::prelude as ui;

impl WorkerProgressViewModel {
    fn snapshot(self) -> ui::ProgressSnapshot {
        ui::ProgressSnapshot::new(self.completed, self.total)
    }
}

pub(in crate::native_app) fn bottom_status_bar(model: StatusBarViewModel) -> ui::View<GuiMessage> {
    ui::status_bar_from_parts(
        ui::StatusBarParts::new(ui::StatusSegments::new(
            selected_sample_count_label(model.selected_sample_count),
            model.status_text,
            "",
        ))
        .left_width(120.0)
        .trailing(worker_progress_bar(
            model.worker_progress,
            model.progress_tick,
        )),
    )
}

fn selected_sample_count_label(count: usize) -> String {
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
}

pub(in crate::native_app) fn worker_progress_bar(
    progress: Option<WorkerProgressViewModel>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    let Some(progress) = progress else {
        return ui::empty().width(0.0).height(10.0);
    };
    let track_width = 180.0;
    let progress_bar = ui::progress_bar_for_snapshot(progress.snapshot(), progress_tick)
        .colors(
            ui::Rgba8::new(48, 50, 51, 210),
            ui::Rgba8::new(255, 112, 86, 210),
        )
        .max_track_height(8.0)
        .activatable();
    progress_bar
        .message(GuiMessage::ToggleJobDetails)
        .key("bottom-status-progress-bar")
        .width(track_width)
        .height(10.0)
}

pub(in crate::native_app) fn job_details_popover(
    progress: &FolderScanProgress,
) -> ui::View<GuiMessage> {
    let total_label =
        ui::ProgressSnapshot::new(progress.completed, progress.total).count_label("found");
    let detail = if progress.detail.is_empty() {
        String::from("Waiting for next item")
    } else {
        progress.detail.clone()
    };
    let content = ui::column([
        ui::text_line(format!("Type: {}", progress.phase), 20.0),
        ui::text_line(format!("Source: {}", progress.label), 20.0),
        ui::text_line(format!("Progress: {total_label}"), 20.0),
        ui::text_line(format!("Current: {detail}"), 20.0),
    ])
    .spacing(5.0)
    .fill_width();
    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::new("Job Details", content)
                .style(ui::WidgetStyle::strong(ui::WidgetTone::Neutral))
                .padding(8.0)
                .spacing(5.0)
                .title_height(22.0),
            ui::Vector2::new(300.0, 132.0),
        )
        .horizontal(ui::LayerHorizontalAnchor::End)
        .vertical(ui::LayerVerticalAnchor::End)
        .inset(14.0, 38.0),
        GuiMessage::CloseJobDetails,
    )
    .key("bottom-job-details-popover")
}
