use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    ActivationInputPolicy, FocusBehavior, PaintBounds, TextWrap, Widget, WidgetCommon, WidgetInput,
    WidgetOutput, WidgetSizing, handle_activation_input,
};

use crate::gui_app::{AUDIO_ENGINE_PILL_HEIGHT, AUDIO_ENGINE_PILL_WIDTH, GuiMessage};

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct AudioEnginePill {
    common: WidgetCommon,
    label: String,
}

impl AudioEnginePill {
    pub(in crate::gui_app) fn new(label: String, active: bool) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(
                AUDIO_ENGINE_PILL_WIDTH,
                AUDIO_ENGINE_PILL_HEIGHT,
            )),
        );
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_state_layers = false;
        common.state.active = active;
        Self { common, label }
    }
}

impl Widget for AudioEnginePill {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        handle_activation_input(
            &mut self.common.state,
            bounds,
            &input,
            ActivationInputPolicy::focusable(),
        )
        .activated()
        .then(|| WidgetOutput::typed(GuiMessage::ToggleAudioSettings))
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let hovered_or_pressed = self.common.state.hovered || self.common.state.pressed;
        paint_background(
            primitives,
            self.common.id,
            bounds,
            self.common.state.active,
            hovered_or_pressed,
        );
        paint_focus_ring(
            primitives,
            self.common.id,
            bounds,
            self.common.state.focused,
        );
        paint_label(primitives, self.common.id, bounds, self.label.as_str());
    }
}

fn paint_background(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    active: bool,
    hovered_or_pressed: bool,
) {
    let fill = if active {
        Rgba8 {
            r: 50,
            g: 54,
            b: 56,
            a: 245,
        }
    } else if hovered_or_pressed {
        Rgba8 {
            r: 42,
            g: 43,
            b: 44,
            a: 245,
        }
    } else {
        Rgba8 {
            r: 31,
            g: 32,
            b: 33,
            a: 235,
        }
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect: bounds,
        color: fill,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: Rect::from_min_max(
            Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
            Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
        ),
        color: Rgba8 {
            r: 78,
            g: 79,
            b: 80,
            a: if hovered_or_pressed { 230 } else { 165 },
        },
        width: 1.0,
    }));
}

fn paint_focus_ring(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    focused: bool,
) {
    if !focused {
        return;
    }
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: Rect::from_min_max(
            Point::new(bounds.min.x - 1.0, bounds.min.y - 1.0),
            Point::new(bounds.max.x + 1.0, bounds.max.y + 1.0),
        ),
        color: Rgba8 {
            r: 255,
            g: 112,
            b: 86,
            a: 190,
        },
        width: 1.0,
    }));
}

fn paint_label(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, bounds: Rect, label: &str) {
    let font_size = 9.0;
    let text_rect = Rect::from_min_max(
        Point::new(bounds.min.x + 5.0, bounds.min.y),
        Point::new(bounds.max.x - 5.0, bounds.max.y),
    );
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: PaintText::from(label),
        rect: text_rect,
        font_size,
        baseline: Some(((text_rect.height() - font_size) * 0.5 + font_size * 0.78).round()),
        color: Rgba8 {
            r: 183,
            g: 184,
            b: 184,
            a: 235,
        },
        align: PaintTextAlign::Center,
        wrap: TextWrap::None,
    }));
}
