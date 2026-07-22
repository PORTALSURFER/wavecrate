mod identity;
mod progress_bar;
mod projection;

#[cfg(test)]
use self::projection::job_details_popover_projection;
use self::projection::{
    JobDetailsPopoverProjection, bottom_status_bar_projection,
    job_details_popover_projection_from_model,
};
#[cfg(test)]
use crate::native_app::app::FolderScanProgress;
use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::palette::{DANGER, WARNING};
#[cfg(test)]
use crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel;
use crate::native_app::app_chrome::view_models::status_bar::{StatusBarViewModel, StatusSeverity};
use crate::native_app::ui::ids::{
    JOB_DETAILS_CURRENT_ROW_ID_BASE, WORKER_PROGRESS_ACTIVITY_OVERLAY_ID, WORKER_PROGRESS_ROOT_ID,
};
use radiant::gui::feedback::horizontal_progress_activity_rect;
use radiant::prelude as ui;
use radiant::runtime::{PaintPrimitive, SurfaceNode, TransientOverlayContext, WidgetPaint};
use radiant::widgets::{TextWidget, WidgetSizing};

const STATUS_BAR_HEIGHT: f32 = 30.0;
const STATUS_BAR_SEGMENT_HEIGHT: f32 = 20.0;
const STATUS_BAR_LEFT_WIDTH: f32 = 120.0;
const STATUS_BAR_LISTING_WIDTH: f32 = 310.0;
const STATUS_BAR_SPACING: f32 = 8.0;
const STATUS_BAR_PADDING_X: f32 = 12.0;
const STATUS_BAR_PADDING_Y: f32 = 4.0;
const JOB_DETAILS_WIDTH: f32 = 680.0;
const JOB_DETAILS_HEIGHT: f32 = 212.0;
const JOB_DETAILS_ROW_HEIGHT: f32 = 20.0;
const JOB_DETAILS_TOOLTIP_THRESHOLD: usize = 72;
const SOURCE_MISSING_STATUS_COLOR: ui::Rgba8 = DANGER.with_alpha(230);
const SOURCE_PROCESSING_ACTIVITY_COLOR: ui::Rgba8 = WARNING.with_alpha(220);
const SOURCE_PROCESSING_ACTIVITY_HEIGHT: f32 = 2.0;
const SOURCE_PROCESSING_ACTIVITY_CYCLES_PER_SECOND: f32 = 2.1;
const SOURCE_PROCESSING_ACTIVITY_SEGMENT_FRACTION: f32 = 0.24;
const SOURCE_PROCESSING_ACTIVITY_MIN_WIDTH: f32 = 18.0;
const SOURCE_PROCESSING_SOURCE_PULSE_ID: u64 = 0x7372_635f_7075_6c73;
const SOURCE_PROCESSING_SOURCE_PULSE_CYCLES_PER_SECOND: f32 = 0.85;

pub(in crate::native_app) fn bottom_status_area(state: &NativeAppState) -> ui::View<GuiMessage> {
    bottom_status_bar(StatusBarViewModel::from_app_state(state))
        .overlays(status_bar_overlays(state))
}

pub(in crate::native_app) fn bottom_status_bar(model: StatusBarViewModel) -> ui::View<GuiMessage> {
    let projection = bottom_status_bar_projection(model);
    ui::row([
        status_segment(projection.selected_sample_count_label).width(STATUS_BAR_LEFT_WIDTH),
        status_segment(projection.listed_audio_count_label).width(STATUS_BAR_LISTING_WIDTH),
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

impl NativeAppState {
    pub(in crate::native_app) fn source_processing_activity_overlay_visible(&self) -> bool {
        self.library.folder_progress().is_none()
            && self.background.normalization_progress.is_none()
            && self.background.file_move_progress.is_none()
            && self.waveform.cache.active_folder_warm_folder_id.is_none()
            && self.background.source_processing_progress.is_some()
    }

    pub(in crate::native_app) fn paint_source_processing_activity_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if !self.source_processing_activity_overlay_visible() {
            return;
        }
        let Some(bounds) = context
            .plan
            .first_widget_rect_by_priority([WORKER_PROGRESS_ROOT_ID])
        else {
            return;
        };
        let height = bounds.height().min(SOURCE_PROCESSING_ACTIVITY_HEIGHT);
        if height <= 0.0 {
            return;
        }
        let y = bounds.min.y + (bounds.height() - height) * 0.5;
        let track = ui::Rect::from_min_max(
            ui::Point::new(bounds.min.x, y),
            ui::Point::new(bounds.max.x, y + height),
        );
        let position = (context.animation_time.as_secs_f32()
            * SOURCE_PROCESSING_ACTIVITY_CYCLES_PER_SECOND)
            .fract();
        let Some(activity) = horizontal_progress_activity_rect(
            track,
            position,
            SOURCE_PROCESSING_ACTIVITY_SEGMENT_FRACTION,
            SOURCE_PROCESSING_ACTIVITY_MIN_WIDTH,
        ) else {
            return;
        };
        WidgetPaint::new(primitives, WORKER_PROGRESS_ACTIVITY_OVERLAY_ID)
            .push_visible_fill_rect(activity, SOURCE_PROCESSING_ACTIVITY_COLOR);
    }

    pub(in crate::native_app) fn paint_source_processing_source_pulse(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(progress) =
            self.background
                .source_processing_progress
                .as_ref()
                .filter(|progress| {
                    progress.active && progress.source_row_active && !progress.source_id.is_empty()
                })
        else {
            return;
        };
        let widget_id =
            crate::native_app::app_chrome::library_browser::library_sidebar::source_row_widget_id(
                &progress.source_id,
            );
        let Some(bounds) = context.plan.first_widget_rect_by_priority([widget_id]) else {
            return;
        };
        let wave = (context.animation_time.as_secs_f32()
            * std::f32::consts::TAU
            * SOURCE_PROCESSING_SOURCE_PULSE_CYCLES_PER_SECOND)
            .sin();
        let alpha = (28.0 + 26.0 * ((wave + 1.0) * 0.5)).round() as u8;
        WidgetPaint::new(primitives, SOURCE_PROCESSING_SOURCE_PULSE_ID)
            .push_visible_fill_rect(bounds, WARNING.with_alpha(alpha));
    }
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
    if state.ui.chrome.job_details_open {
        let model = StatusBarViewModel::from_app_state(state);
        if let Some(details) = model.job_details {
            return Some(job_details_popover_from_projection(
                job_details_popover_projection_from_model(details),
            ));
        }
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

#[cfg(test)]
pub(in crate::native_app) fn job_details_popover(
    progress: &FolderScanProgress,
) -> ui::View<GuiMessage> {
    job_details_popover_from_projection(job_details_popover_projection(progress))
}

fn job_details_popover_from_projection(
    projection: JobDetailsPopoverProjection,
) -> ui::View<GuiMessage> {
    let source_scan_recovery = projection.source_scan_recovery;
    let [kind, source, progress, current] = projection.rows;
    let mut rows = vec![
        ui::text_line(kind, JOB_DETAILS_ROW_HEIGHT),
        ui::text_line(source, JOB_DETAILS_ROW_HEIGHT),
        ui::text_line(progress, JOB_DETAILS_ROW_HEIGHT),
    ];
    let current = current
        .strip_prefix("Current: ")
        .unwrap_or(current.as_str());
    rows.extend(
        current
            .split(" | ")
            .enumerate()
            .map(job_details_current_row),
    );
    if source_scan_recovery {
        rows.push(
            ui::button_row([
                ui::button("Retry")
                    .primary()
                    .message(GuiMessage::RetryActiveSourceScan)
                    .width(78.0),
                ui::button("Cancel")
                    .message(GuiMessage::CancelActiveSourceScan)
                    .width(78.0),
            ])
            .into(),
        );
    }
    let content = ui::column(rows).spacing(5.0).fill_width();
    radiant::application::closeable_dialog_layer_from_parts(
        radiant::application::DialogLayerParts::new(
            projection.title,
            content,
            ui::WidgetTone::Neutral,
            ui::Vector2::new(JOB_DETAILS_WIDTH, JOB_DETAILS_HEIGHT),
        )
        .horizontal(ui::LayerHorizontalAnchor::End)
        .vertical(ui::LayerVerticalAnchor::End)
        .inset(14.0, 38.0),
        GuiMessage::CloseJobDetails,
    )
    .key(identity::JOB_DETAILS_POPOVER_KEY)
}

fn job_details_current_row((index, full_part): (usize, &str)) -> ui::View<GuiMessage> {
    let compact_part = if index == 0 {
        full_part.to_string()
    } else {
        compact_job_detail(full_part)
    };
    let display = if index == 0 {
        format!("Current: {compact_part}")
    } else {
        format!("         {compact_part}")
    };
    let full = if index == 0 {
        format!("Current: {full_part}")
    } else {
        full_part.to_string()
    };
    let show_tooltip = display != full || full.chars().count() > JOB_DETAILS_TOOLTIP_THRESHOLD;
    if show_tooltip {
        let mut row = TextWidget::new(
            JOB_DETAILS_CURRENT_ROW_ID_BASE + index as u64,
            display,
            WidgetSizing::new(
                ui::Vector2::new(1.0, JOB_DETAILS_ROW_HEIGHT),
                ui::Vector2::new(JOB_DETAILS_WIDTH - 48.0, JOB_DETAILS_ROW_HEIGHT),
            ),
        );
        row.common = row.common.with_pointer_focus();
        row.common.tooltip = Some(full);
        ui::View::from(SurfaceNode::static_widget(row))
    } else {
        ui::text_line(display, JOB_DETAILS_ROW_HEIGHT)
            .truncate()
            .id(JOB_DETAILS_CURRENT_ROW_ID_BASE + index as u64)
    }
}

fn compact_job_detail(detail: &str) -> String {
    let Some((label, value)) = detail.split_once(": ") else {
        return detail.to_string();
    };
    let Some(file_name) = value
        .rsplit(['/', '\\'])
        .next()
        .filter(|file_name| !file_name.is_empty() && *file_name != value)
    else {
        return detail.to_string();
    };
    format!("{label}: {file_name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;
    use radiant::runtime::{DeclarativeOwnedRuntimeBridge, Event, SurfaceRuntime};
    use std::sync::Arc;

    #[test]
    fn bottom_status_bar_paints_missing_source_status_as_warning() {
        let frame = bottom_status_bar(StatusBarViewModel {
            selected_sample_count: 0,
            listed_audio_count: 0,
            listing_includes_subfolders: false,
            status_text: String::from("Source missing | Ready"),
            status_severity: StatusSeverity::Warning,
            worker_progress: None,
            job_details: None,
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

    #[test]
    fn job_details_popover_keeps_all_current_work_inside_dialog() {
        let current =
            "Current: Analyzing audio | samples: long/path/to/current-file.wav | 2 sources active";
        let viewport = ui::Vector2::new(540.0, 260.0);
        let frame = job_details_popover_from_projection(JobDetailsPopoverProjection {
            title: "Job Details",
            rows: [
                String::from("Type: Source processing"),
                String::from("Source: Projects"),
                String::from("Progress: 61/200"),
                String::from(current),
            ],
            source_scan_recovery: false,
        })
        .view_frame_at_size_with_default_theme(viewport);
        let current_run = frame
            .paint_plan
            .first_text_run("         2 sources active")
            .expect("final current work detail should paint");
        let dialog_bottom = viewport.y - 38.0;

        assert!(
            current_run.rect.max.y <= dialog_bottom,
            "current work row must remain inside the bottom-anchored dialog"
        );
    }

    #[test]
    fn stalled_source_scan_popover_exposes_retry_and_cancel_actions() {
        let viewport = ui::Vector2::new(540.0, 260.0);
        let frame = job_details_popover_from_projection(JobDetailsPopoverProjection {
            title: "Job Details",
            rows: [
                String::from("Type: Source scan"),
                String::from("Source: Samples"),
                String::from("Progress: Taking longer than expected"),
                String::from("Current: Waiting for database access"),
            ],
            source_scan_recovery: true,
        })
        .view_frame_at_size_with_default_theme(viewport);

        assert!(frame.paint_plan.contains_text("Retry"));
        assert!(frame.paint_plan.contains_text("Cancel"));
        let cancel = frame
            .paint_plan
            .first_text_run("Cancel")
            .expect("cancel action paints");
        assert!(cancel.rect.max.y <= viewport.y - 38.0);
    }

    #[test]
    fn job_details_compacts_long_paths_and_keeps_the_full_value_in_a_tooltip() {
        let full_path =
            "Projects: OLD/#sounddesign/master-recordings/2026/#sounddesign-long-recording.wav";
        let row = job_details_current_row((1, full_path));
        let surface = row.into_surface();
        let widget = surface
            .find_widget(JOB_DETAILS_CURRENT_ROW_ID_BASE + 1)
            .expect("current work row should be addressable");

        assert_eq!(
            widget.widget_object().common().tooltip.as_deref(),
            Some(full_path)
        );
        assert_eq!(
            widget.widget_object().common().focus,
            radiant::widgets::FocusBehavior::Pointer
        );
        let frame = surface.frame_at_size_with_default_theme(ui::Vector2::new(680.0, 30.0));
        assert!(
            frame
                .paint_plan
                .contains_text("         Projects: #sounddesign-long-recording.wav")
        );
        assert!(!frame.paint_plan.contains_text(full_path));
    }

    #[test]
    fn compact_job_detail_handles_windows_paths() {
        assert_eq!(
            compact_job_detail(r"samples: drums\processed\kick.wav"),
            "samples: kick.wav"
        );
    }

    #[test]
    fn compact_job_detail_paints_wrapped_tooltip_when_hovered() {
        let full_path = "Projects: old/recording.wav";
        let compact = "         Projects: recording.wav";
        let bridge = DeclarativeOwnedRuntimeBridge::new(
            (),
            |_| {
                radiant::runtime::UiSurface::new(
                    job_details_current_row((1, full_path)).into_node(),
                )
            },
            |_, _: GuiMessage| {},
        );
        let mut runtime = SurfaceRuntime::new(bridge, ui::Vector2::new(680.0, 80.0));
        let initial = runtime.frame_with_default_theme();
        let compact_rect = initial
            .paint_plan
            .first_text_run(compact)
            .expect("compact path should paint")
            .rect;

        runtime.dispatch_event(Event::PointerMove {
            position: compact_rect.center(),
        });

        assert!(
            runtime
                .frame_with_default_theme()
                .paint_plan
                .contains_text(full_path),
            "hovering the compact path should paint its complete value"
        );
    }
}
