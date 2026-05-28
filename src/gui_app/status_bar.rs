use super::{FolderScanProgress, GuiAppState, GuiMessage, NormalizationProgress};
use radiant::prelude as ui;

mod progress_bar;
pub(super) use progress_bar::StatusProgressBar;

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
        return if progress.total == 0 {
            format!(
                "{} {} | {} items found",
                progress.phase, progress.label, progress.completed
            )
        } else {
            format!(
                "{} {} | {}/{} | {}",
                progress.phase,
                progress.label,
                progress.completed.min(progress.total),
                progress.total,
                progress.detail
            )
        };
    }
    state
        .normalization_progress
        .as_ref()
        .map(|progress| {
            if progress.total == 0 {
                format!("Normalizing {} | {}", progress.label, progress.detail)
            } else {
                format!(
                    "Normalizing {} | {}/{} | {}",
                    progress.label,
                    progress.completed.min(progress.total),
                    progress.total,
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
    let progress_bar = if progress.total() == 0 {
        StatusProgressBar::indeterminate(state.progress_tick)
    } else {
        StatusProgressBar::determinate(progress.completed() as f32 / progress.total().max(1) as f32)
    };
    ui::custom_widget(progress_bar, |output| {
        output.typed_ref::<GuiMessage>().cloned()
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
    fn completed(&self) -> usize {
        match self {
            Self::Folder(progress) => progress.completed,
            Self::Normalization(progress) => progress.completed,
        }
    }

    fn total(&self) -> usize {
        match self {
            Self::Folder(progress) => progress.total,
            Self::Normalization(progress) => progress.total,
        }
    }
}

pub(super) fn job_details_popover(progress: &FolderScanProgress) -> ui::View<GuiMessage> {
    let total_label = if progress.total == 0 {
        format!("{} found", progress.completed)
    } else {
        format!(
            "{}/{}",
            progress.completed.min(progress.total),
            progress.total
        )
    };
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
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Neutral,
        prominence: ui::WidgetProminence::Strong,
    })
    .spacing(5.0)
    .padding(8.0)
    .width(300.0)
    .height(132.0);
    ui::column([
        ui::spacer().fill_height(),
        ui::row([ui::spacer().fill_width(), panel])
            .padding_x(14.0)
            .padding_y(38.0)
            .fill_width()
            .height(172.0),
    ])
    .fill()
}
