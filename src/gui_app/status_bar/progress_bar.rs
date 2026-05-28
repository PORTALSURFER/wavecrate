use super::GuiMessage;
use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::runtime::{PaintFillRect, PaintPrimitive};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, PointerButton, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
};

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct StatusProgressBar {
    common: WidgetCommon,
    mode: StatusProgressMode,
}

#[derive(Clone, Copy, Debug)]
enum StatusProgressMode {
    Determinate(f32),
    Indeterminate(f32),
}

impl StatusProgressBar {
    pub(in crate::gui_app) fn determinate(fraction: f32) -> Self {
        Self::new(StatusProgressMode::Determinate(fraction.clamp(0.0, 1.0)))
    }

    pub(super) fn indeterminate(tick: f32) -> Self {
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
        let track_height = bounds.height().clamp(0.0, 8.0);
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
