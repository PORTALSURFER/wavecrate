use crate::native_app::app_scope::{
    FolderScanProgress, GuiMessage, NativeAppState, NormalizationProgress,
};
use radiant::prelude as ui;

pub(in crate::native_app) fn bottom_status_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    ui::status_bar_from_parts(
        ui::StatusBarParts::new(ui::StatusSegments::new(
            selected_sample_count_label(state),
            bottom_status_text(state),
            "",
        ))
        .left_width(120.0)
        .trailing(worker_progress_bar(state)),
    )
}

fn selected_sample_count_label(state: &NativeAppState) -> String {
    let count = state.folder_browser.selected_audio_file_count();
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
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

pub(in crate::native_app) fn worker_progress_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    let Some(progress) = active_worker_progress(state) else {
        return ui::empty().width(0.0).height(10.0);
    };
    let track_width = 180.0;
    let snapshot = progress.snapshot();
    let progress_bar = ui::progress_bar_for_snapshot(snapshot, state.progress_tick)
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

fn active_worker_progress(state: &NativeAppState) -> Option<WorkerProgressView<'_>> {
    state
        .folder_progress
        .as_ref()
        .map(WorkerProgressView::Folder)
        .or_else(|| {
            state
                .normalization_progress
                .as_ref()
                .map(WorkerProgressView::Normalization)
        })
}

enum WorkerProgressView<'a> {
    Folder(&'a FolderScanProgress),
    Normalization(&'a NormalizationProgress),
}

impl WorkerProgressView<'_> {
    fn snapshot(&self) -> ui::ProgressSnapshot {
        match self {
            Self::Folder(progress) => ui::ProgressSnapshot::new(progress.completed, progress.total),
            Self::Normalization(progress) => {
                ui::ProgressSnapshot::new(progress.completed, progress.total)
            }
        }
    }
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
