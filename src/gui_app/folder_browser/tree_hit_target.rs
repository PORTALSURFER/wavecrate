use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    runtime::{PaintFillRect, PaintPrimitive, PaintStrokeRect},
    theme::ThemeTokens,
    widgets::{
        DragHandleMessage, FocusBehavior, PaintBounds, PointerButton, Widget, WidgetCommon,
        WidgetInput, WidgetOutput, WidgetSizing,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum FolderTreeHitMessage {
    Activate,
    ContextMenu(Point),
    Drag(DragHandleMessage),
    Drop,
    HoverDropTarget,
}

#[derive(Clone, Debug)]
pub(super) struct FolderTreeHitTarget {
    common: WidgetCommon,
    selected: bool,
    drop_target: bool,
    drag_active: bool,
    drop_candidate: bool,
    dragged: bool,
}

impl FolderTreeHitTarget {
    pub(super) fn new(
        selected: bool,
        drop_target: bool,
        drag_active: bool,
        drop_candidate: bool,
    ) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            selected,
            drop_target,
            drag_active,
            drop_candidate,
            dragged: false,
        }
    }
}

impl Widget for FolderTreeHitTarget {
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
                Some(WidgetOutput::typed(FolderTreeHitMessage::ContextMenu(
                    position,
                )))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => self.handle_primary_release(bounds, position),
            WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => Some(WidgetOutput::typed(FolderTreeHitMessage::Drop)),
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
        self.paint_background(primitives, bounds);
        self.paint_drop_target_outline(primitives, bounds);
    }
}

impl FolderTreeHitTarget {
    fn handle_pointer_move(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        if self.common.state.pressed {
            let message = if self.dragged || self.drag_active {
                DragHandleMessage::Moved { position }
            } else {
                self.dragged = true;
                DragHandleMessage::Started { position }
            };
            return Some(WidgetOutput::typed(FolderTreeHitMessage::Drag(message)));
        }
        if self.common.state.hovered && self.drag_active {
            return Some(WidgetOutput::typed(FolderTreeHitMessage::HoverDropTarget));
        }
        None
    }

    fn handle_primary_release(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        let activated = self.common.state.pressed && !self.dragged && bounds.contains(position);
        let dragged = self.common.state.pressed && (self.dragged || self.drag_active);
        self.common.state.pressed = false;
        self.common.state.hovered = bounds.contains(position);
        self.dragged = false;
        if dragged {
            return Some(WidgetOutput::typed(FolderTreeHitMessage::Drag(
                DragHandleMessage::Ended { position },
            )));
        }
        activated.then(|| WidgetOutput::typed(FolderTreeHitMessage::Activate))
    }

    fn paint_background(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        let fill = if self.drop_target {
            Some(Rgba8 {
                r: 255,
                g: 130,
                b: 78,
                a: 150,
            })
        } else if self.common.state.hovered && self.drop_candidate {
            Some(Rgba8 {
                r: 255,
                g: 122,
                b: 74,
                a: 110,
            })
        } else if self.common.state.pressed || self.common.state.hovered {
            Some(Rgba8 {
                r: 255,
                g: 110,
                b: 85,
                a: if self.common.state.pressed { 120 } else { 80 },
            })
        } else if self.selected {
            Some(Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 105,
            })
        } else {
            None
        };
        if let Some(color) = fill {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color,
            }));
        }
    }

    fn paint_drop_target_outline(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.drop_target {
            return;
        }
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
                Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
            ),
            color: Rgba8 {
                r: 255,
                g: 180,
                b: 130,
                a: 210,
            },
            width: 1.0,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::widgets::PointerModifiers;

    fn message_from(output: Option<WidgetOutput>) -> FolderTreeHitMessage {
        *output
            .expect("expected widget output")
            .typed_ref::<FolderTreeHitMessage>()
            .expect("expected folder tree message")
    }

    #[test]
    fn active_drag_survives_widget_refresh_as_moved() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut first = FolderTreeHitTarget::new(false, false, false, false);
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
            FolderTreeHitMessage::Drag(DragHandleMessage::Started {
                position: Point::new(16.0, 7.0),
            })
        );

        let mut refreshed = FolderTreeHitTarget::new(false, false, true, false);
        refreshed.common.state = first.common.state;
        assert_eq!(
            message_from(refreshed.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(34.0, 8.0),
                },
            )),
            FolderTreeHitMessage::Drag(DragHandleMessage::Moved {
                position: Point::new(34.0, 8.0),
            })
        );
    }

    #[test]
    fn active_drag_survives_widget_refresh_until_release() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut refreshed = FolderTreeHitTarget::new(false, false, true, false);
        refreshed.common.state.pressed = true;

        assert_eq!(
            message_from(refreshed.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(90.0, 9.0),
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                },
            )),
            FolderTreeHitMessage::Drag(DragHandleMessage::Ended {
                position: Point::new(90.0, 9.0),
            })
        );
    }
}
