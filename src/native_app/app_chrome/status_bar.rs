use crate::native_app::app::{FolderScanProgress, GuiMessage, NativeAppState};
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::view_models::status_bar::{
    StatusBarViewModel, WorkerProgressViewModel,
};
use radiant::prelude as ui;

impl WorkerProgressViewModel {
    fn snapshot(self) -> ui::ProgressSnapshot {
        ui::ProgressSnapshot::new(self.completed, self.total)
    }
}

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
    ui::status_bar_from_parts(
        ui::StatusBarParts::new(ui::StatusSegments::left_center(
            selected_sample_count_label(model.selected_sample_count),
            model.status_text,
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
        return ui::empty().width(0.0).height(WORKER_PROGRESS_HEIGHT);
    };
    if progress.active_animation {
        return source_cache_worker_progress(progress, progress_tick);
    }
    overall_progress_bar(progress, progress_tick)
        .key("bottom-status-progress-bar")
        .width(WORKER_PROGRESS_TRACK_WIDTH)
        .height(WORKER_PROGRESS_HEIGHT)
}

fn source_cache_worker_progress(
    progress: WorkerProgressViewModel,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    ui::column([
        overall_progress_bar(progress, progress_tick)
            .key("bottom-status-progress-overall")
            .width(WORKER_PROGRESS_TRACK_WIDTH)
            .height(OVERALL_PROGRESS_HEIGHT),
        active_cache_progress_bar(progress.current_fraction, progress_tick)
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
    progress: WorkerProgressViewModel,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    ui::progress_bar_for_snapshot(progress.snapshot(), progress_tick)
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
    ui::closeable_dialog_layer_from_parts(
        ui::DialogLayerParts::new(
            "Job Details",
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
