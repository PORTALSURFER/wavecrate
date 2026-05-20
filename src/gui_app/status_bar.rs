use super::{FolderScanProgress, GuiAppState, GuiMessage};
use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::prelude as ui;
use radiant::runtime::{PaintFillRect, PaintPrimitive};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, PointerButton, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
};

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
    state
        .folder_progress
        .as_ref()
        .map(|progress| {
            if progress.total == 0 {
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
            }
        })
        .unwrap_or_else(|| state.sample_status.clone())
}

pub(super) fn worker_progress_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    let Some(progress) = state.folder_progress.as_ref() else {
        return ui::text("").width(0.0).height(10.0);
    };
    let track_width = 180.0;
    let progress_bar = if progress.total == 0 {
        StatusProgressBar::indeterminate(state.progress_tick)
    } else {
        StatusProgressBar::determinate(progress.completed as f32 / progress.total.max(1) as f32)
    };
    ui::custom_widget(progress_bar, |output| {
        output.typed_ref::<GuiMessage>().cloned()
    })
    .key("bottom-status-progress-bar")
    .width(track_width)
    .height(10.0)
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

#[derive(Clone, Debug)]
pub(super) struct StatusProgressBar {
    common: WidgetCommon,
    mode: StatusProgressMode,
}

#[derive(Clone, Copy, Debug)]
enum StatusProgressMode {
    Determinate(f32),
    Indeterminate(f32),
}

impl StatusProgressBar {
    pub(super) fn determinate(fraction: f32) -> Self {
        Self::new(StatusProgressMode::Determinate(fraction.clamp(0.0, 1.0)))
    }

    fn indeterminate(tick: f32) -> Self {
        Self::new(StatusProgressMode::Indeterminate(tick.rem_euclid(1.0)))
    }

    fn new(mode: StatusProgressMode) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::Pointer;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common, mode }
    }
}

impl Widget for StatusProgressBar {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                activated.then(|| WidgetOutput::typed(GuiMessage::ToggleJobDetails))
            }
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                }
                None
            }
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let track_height = bounds.height().min(8.0).max(0.0);
        let track_top = bounds.min.y + (bounds.height() - track_height) * 0.5;
        let track = Rect::from_min_max(
            Point::new(bounds.min.x, track_top),
            Point::new(bounds.max.x, track_top + track_height),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: track,
            color: Rgba8 {
                r: 48,
                g: 50,
                b: 51,
                a: 210,
            },
        }));
        let Some(fill) = status_progress_fill_rect(track, self.mode) else {
            return;
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: fill,
            color: Rgba8 {
                r: 255,
                g: 112,
                b: 86,
                a: 210,
            },
        }));
    }
}

fn status_progress_fill_rect(track: Rect, mode: StatusProgressMode) -> Option<Rect> {
    match mode {
        StatusProgressMode::Determinate(fraction) => {
            if fraction <= 0.0 {
                return None;
            }
            let fill_width = (track.width() * fraction).clamp(1.0, track.width());
            Some(Rect::from_min_max(
                track.min,
                Point::new(track.min.x + fill_width, track.max.y),
            ))
        }
        StatusProgressMode::Indeterminate(tick) => {
            let segment_width = (track.width() * 0.32).clamp(1.0, track.width());
            let travel = (track.width() - segment_width).max(0.0);
            let start = track.min.x + travel * tick.rem_euclid(1.0);
            Some(Rect::from_min_max(
                Point::new(start, track.min.y),
                Point::new(start + segment_width, track.max.y),
            ))
        }
    }
}
