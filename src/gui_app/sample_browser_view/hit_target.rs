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
    drag_active: bool,
    drag_source: bool,
    dragged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) enum SampleFileHitMessage {
    Activate(PointerModifiers),
    ContextMenu(Point),
    Drag(DragHandleMessage),
}

impl SampleFileHitTarget {
    pub(in crate::gui_app) fn new(selected: bool, drag_active: bool, drag_source: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            selected,
            drag_active,
            drag_source,
            dragged: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message_from(output: Option<WidgetOutput>) -> SampleFileHitMessage {
        *output
            .expect("expected widget output")
            .typed_ref::<SampleFileHitMessage>()
            .expect("expected sample file message")
    }

    #[test]
    fn active_drag_uses_runtime_preview_after_widget_refresh() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut first = SampleFileHitTarget::new(false, false, false);
        first.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(6.0, 6.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );
        assert_eq!(
            message_from(first.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(16.0, 7.0),
                },
            )),
            SampleFileHitMessage::Drag(DragHandleMessage::Started {
                position: Point::new(16.0, 7.0),
            })
        );

        let mut refreshed = SampleFileHitTarget::new(false, true, true);
        refreshed.common.state = first.common.state;
        assert!(
            refreshed
                .handle_input(
                    bounds,
                    WidgetInput::PointerMove {
                        position: Point::new(34.0, 8.0),
                    },
                )
                .is_none(),
            "runtime drag preview already tracks pointer movement without app refresh"
        );
    }

    #[test]
    fn active_drag_source_does_not_depend_on_retained_pressed_state() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut refreshed = SampleFileHitTarget::new(false, true, true);

        assert!(
            refreshed
                .handle_input(
                    bounds,
                    WidgetInput::PointerMove {
                        position: Point::new(34.0, 8.0),
                    },
                )
                .is_none(),
            "active drag moves are runtime-local and should not require retained pressed state"
        );
        assert_eq!(
            message_from(refreshed.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(220.0, 90.0),
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                },
            )),
            SampleFileHitMessage::Drag(DragHandleMessage::Ended {
                position: Point::new(220.0, 90.0),
            })
        );
    }

    #[test]
    fn active_drag_non_source_rows_do_not_keep_hover_highlight() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut target = SampleFileHitTarget::new(false, true, false);
        target.common.state.hovered = true;

        assert!(
            target
                .handle_input(
                    bounds,
                    WidgetInput::PointerMove {
                        position: Point::new(34.0, 8.0),
                    },
                )
                .is_none()
        );
        assert!(
            !target.common.state.hovered,
            "sample rows should not retain hover while another file is being dragged"
        );

        let mut primitives = Vec::new();
        target.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            !primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color.a == 155)),
            "non-source rows should not paint hover highlights during active file drags"
        );
    }

    #[test]
    fn hover_state_survives_retained_widget_refresh() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut previous = SampleFileHitTarget::new(false, false, false);
        previous.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(34.0, 8.0),
            },
        );
        assert!(previous.common.state.hovered);

        let mut refreshed = SampleFileHitTarget::new(false, false, false);
        refreshed.synchronize_from_previous(&previous);

        assert!(
            refreshed.common.state.hovered,
            "sample row hover paint should not blink off between retained projections"
        );
        let mut primitives = Vec::new();
        refreshed.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color.a == 155)),
            "refreshed hovered row should keep painting the hover highlight"
        );
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

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
        self.dragged = previous.dragged;
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
        if self.drag_active && !self.drag_source {
            self.common.state.hovered = false;
            return None;
        }
        self.common.state.hovered = bounds.contains(position);
        if self.drag_active && self.drag_source {
            return None;
        }
        if !self.common.state.pressed && !self.drag_source {
            return None;
        }
        let message = if self.dragged || self.drag_active {
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
        let dragged =
            self.drag_source || (self.common.state.pressed && (self.dragged || self.drag_active));
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
        if self.drag_active && !self.drag_source {
            return;
        }
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
