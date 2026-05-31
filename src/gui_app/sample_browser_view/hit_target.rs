use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::runtime::{PaintFillRect, PaintPrimitive};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    DragHandleMessage, FocusBehavior, InteractiveRowMessage, InteractiveRowWidget, PaintBounds,
    PointerModifiers, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
};

const HOVER_FILL: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 24,
};
const PRESSED_FILL: Rgba8 = Rgba8 {
    r: 255,
    g: 108,
    b: 88,
    a: 170,
};

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct SampleFileHitTarget {
    row: InteractiveRowWidget,
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
    suppress_hover: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) enum SampleFileHitMessage {
    Activate(PointerModifiers),
    ContextMenu(Point),
    Drag(DragHandleMessage),
}

impl SampleFileHitTarget {
    pub(in crate::gui_app) fn new(
        selected: bool,
        drag_active: bool,
        drag_source: bool,
        cached: bool,
        suppress_hover: bool,
    ) -> Self {
        let mut row = InteractiveRowWidget::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)))
            .with_drag()
            .with_drag_active(drag_active)
            .with_drag_source(drag_source)
            .suppress_hover(suppress_hover)
            .clear_hover_on_sync()
            .with_activation_modifiers();
        row.common.focus = FocusBehavior::None;
        row.common.paint.bounds = PaintBounds::ClipToRect;
        row.common.paint.paints_focus = false;
        row.common.paint.paints_state_layers = false;
        Self {
            row,
            selected,
            drag_active,
            drag_source,
            cached,
            suppress_hover,
        }
    }
}

impl Widget for SampleFileHitTarget {
    fn common(&self) -> &WidgetCommon {
        &self.row.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.row.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.row
            .handle_input(bounds, input)
            .and_then(|message| self.map_row_message(message))
            .map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.row.synchronize_from_previous(&previous.row);
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.paint_selection_fill(primitives, bounds);
        self.paint_interaction_fill(primitives, bounds);
        self.paint_loaded_marker(primitives, bounds);
        self.paint_selection_marker(primitives, bounds);
    }
}

impl SampleFileHitTarget {
    /// Maps generic Radiant row interactions into sample-browser hit messages.
    fn map_row_message(&self, message: InteractiveRowMessage) -> Option<SampleFileHitMessage> {
        match message {
            InteractiveRowMessage::Activate => {
                Some(SampleFileHitMessage::Activate(PointerModifiers::default()))
            }
            InteractiveRowMessage::ActivateWithModifiers { modifiers } => {
                Some(SampleFileHitMessage::Activate(modifiers))
            }
            InteractiveRowMessage::DoubleActivate => {
                Some(SampleFileHitMessage::Activate(PointerModifiers::default()))
            }
            InteractiveRowMessage::SecondaryActivate { position } => {
                Some(SampleFileHitMessage::ContextMenu(position))
            }
            InteractiveRowMessage::Drag(message) => Some(SampleFileHitMessage::Drag(message)),
            InteractiveRowMessage::Drop | InteractiveRowMessage::HoverDropTarget { .. } => None,
        }
    }

    fn paint_selection_fill(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.selected {
            return;
        }
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.row.common.id,
            rect: bounds,
            color: Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120,
            },
        }));
    }

    fn paint_loaded_marker(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.cached || self.selected {
            return;
        }
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.row.common.id,
            rect: Rect::from_min_size(
                Point::new(bounds.max.x - 3.0, bounds.min.y + 3.0),
                Vector2::new(2.0, (bounds.height() - 6.0).max(8.0)),
            ),
            color: Rgba8 {
                r: 226,
                g: 226,
                b: 226,
                a: 210,
            },
        }));
    }

    fn paint_interaction_fill(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if self.suppress_hover {
            return;
        }
        if self.drag_active && !self.drag_source {
            return;
        }
        if !self.row.common.state.pressed && !self.row.common.state.hovered {
            return;
        }
        let color = if self.row.common.state.pressed {
            PRESSED_FILL
        } else {
            HOVER_FILL
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.row.common.id,
            rect: bounds,
            color,
        }));
    }

    fn paint_selection_marker(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.selected {
            return;
        }
        let marker_height = (bounds.height() - 8.0).max(8.0).min(bounds.height());
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.row.common.id,
            rect: Rect::from_min_size(
                Point::new(
                    bounds.min.x + 1.0,
                    bounds.min.y + (bounds.height() - marker_height) * 0.5,
                ),
                Vector2::new(3.0, marker_height),
            ),
            color: Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 245,
            },
        }));
    }
}

#[cfg(test)]
#[path = "hit_target_tests.rs"]
mod tests;
