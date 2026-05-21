use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::runtime::{PaintFillRect, PaintPrimitive};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    DragHandleMessage, FocusBehavior, PaintBounds, PointerButton, PointerModifiers, Widget,
    WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
};

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct SampleFileHitTarget {
    common: WidgetCommon,
    selected: bool,
    dragged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) enum SampleFileHitMessage {
    Activate(PointerModifiers),
    ContextMenu(Point),
    Drag(DragHandleMessage),
}

impl SampleFileHitTarget {
    pub(in crate::gui_app) fn new(selected: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            selected,
            dragged: false,
        }
    }
}

impl Widget for SampleFileHitTarget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => self.handle_pointer_move(bounds, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.dragged = false;
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = false;
                self.dragged = false;
                Some(WidgetOutput::typed(SampleFileHitMessage::ContextMenu(
                    position,
                )))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers,
            } => self.handle_primary_release(bounds, position, modifiers),
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                    self.dragged = false;
                }
                None
            }
        }
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
        self.paint_selection_fill(primitives, bounds);
        self.paint_interaction_fill(primitives, bounds);
        self.paint_selection_marker(primitives, bounds);
    }
}

impl SampleFileHitTarget {
    fn handle_pointer_move(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        if !self.common.state.pressed {
            return None;
        }
        let message = if self.dragged {
            DragHandleMessage::Moved { position }
        } else {
            self.dragged = true;
            DragHandleMessage::Started { position }
        };
        Some(WidgetOutput::typed(SampleFileHitMessage::Drag(message)))
    }

    fn handle_primary_release(
        &mut self,
        bounds: Rect,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let activated = self.common.state.pressed && !self.dragged && bounds.contains(position);
        let dragged = self.common.state.pressed && self.dragged;
        self.common.state.pressed = false;
        self.common.state.hovered = bounds.contains(position);
        self.dragged = false;
        if dragged {
            return Some(WidgetOutput::typed(SampleFileHitMessage::Drag(
                DragHandleMessage::Ended { position },
            )));
        }
        activated.then(|| WidgetOutput::typed(SampleFileHitMessage::Activate(modifiers)))
    }

    fn paint_selection_fill(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.selected {
            return;
        }
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120,
            },
        }));
    }

    fn paint_interaction_fill(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.common.state.pressed && !self.common.state.hovered {
            return;
        }
        let alpha = if self.common.state.pressed { 170 } else { 155 };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: Rgba8 {
                r: 255,
                g: 108,
                b: 88,
                a: alpha,
            },
        }));
    }

    fn paint_selection_marker(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.selected {
            return;
        }
        let marker_height = (bounds.height() - 8.0).max(8.0).min(bounds.height());
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
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
