use super::{FolderScanProgress, GuiAppState, GuiMessage, NormalizationProgress};
use radiant::prelude as ui;

pub(super) fn bottom_status_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::row([
        ui::text(selected_sample_count_label(state))
            .height(20.0)
            .width(120.0),
        ui::text(bottom_status_text(state))
            .height(20.0)
            .fill_width(),
        worker_progress_bar(state),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn selected_sample_count_label(state: &GuiAppState) -> String {
    let count = state.folder_browser.selected_audio_file_count();
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
}

fn bottom_status_text(state: &GuiAppState) -> String {
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

pub(super) fn worker_progress_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    let Some(progress) = active_worker_progress(state) else {
        return ui::text("").width(0.0).height(10.0);
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
        .mapped(|message| match message {
            ui::ProgressBarMessage::Activate => GuiMessage::ToggleJobDetails,
        })
        .key("bottom-status-progress-bar")
        .width(track_width)
        .height(10.0)
}

fn active_worker_progress(state: &GuiAppState) -> Option<WorkerProgressView<'_>> {
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

pub(super) fn job_details_popover(progress: &FolderScanProgress) -> ui::View<GuiMessage> {
    let total_label =
        ui::ProgressSnapshot::new(progress.completed, progress.total).count_label("found");
    let detail = if progress.detail.is_empty() {
        String::from("Waiting for next item")
    } else {
        progress.detail.clone()
    };
    let panel = ui::column([
        ui::row([
            ui::text("Job Details").height(20.0).fill_width(),
            ui::button("x")
                .subtle()
                .message(GuiMessage::CloseJobDetails)
                .width(24.0)
                .height(20.0),
        ])
        .height(22.0)
        .fill_width(),
        ui::text(format!("Type: {}", progress.phase))
            .height(20.0)
            .fill_width()
            .truncate(),
        ui::text(format!("Source: {}", progress.label))
            .height(20.0)
            .fill_width()
            .truncate(),
        ui::text(format!("Progress: {total_label}"))
            .height(20.0)
            .fill_width()
            .truncate(),
        ui::text(format!("Current: {detail}"))
            .height(20.0)
            .fill_width()
            .truncate(),
    ])
    .key("bottom-job-details-popover")
    .style(ui::WidgetStyle::new(
        ui::WidgetTone::Neutral,
        ui::WidgetProminence::Strong,
    ))
    .spacing(5.0)
    .padding(8.0)
    .width(300.0)
    .height(132.0);
    ui::anchored_layer(
        panel,
        ui::Vector2::new(300.0, 132.0),
        ui::LayerHorizontalAnchor::End,
        ui::LayerVerticalAnchor::End,
        14.0,
        38.0,
    )
}
