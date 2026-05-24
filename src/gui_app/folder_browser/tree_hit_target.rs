use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    runtime::{
        PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
    },
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
    HoverDropTarget(Point),
}

#[derive(Clone, Debug)]
pub(super) struct FolderTreeHitTarget {
    common: WidgetCommon,
    label: PaintText,
    selected: bool,
    drop_target: bool,
    drag_active: bool,
    drag_source: bool,
    drop_candidate: bool,
    drop_target_active: bool,
    dragged: bool,
}

impl FolderTreeHitTarget {
    pub(super) fn new(
        label: impl Into<PaintText>,
        selected: bool,
        drop_target: bool,
        drag_active: bool,
        drag_source: bool,
        drop_candidate: bool,
        drop_target_active: bool,
    ) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            label: label.into(),
            selected,
            drop_target,
            drag_active,
            drag_source,
            drop_candidate,
            drop_target_active,
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
        self.paint_label(primitives, bounds, _theme);
    }
}

impl FolderTreeHitTarget {
    fn handle_pointer_move(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        if self.common.state.pressed || self.drag_source {
            let message = if self.dragged || self.drag_active {
                DragHandleMessage::Moved { position }
            } else {
                self.dragged = true;
                DragHandleMessage::Started { position }
            };
            return Some(WidgetOutput::typed(FolderTreeHitMessage::Drag(message)));
        }
        if self.common.state.hovered
            && self.drag_active
            && !self.drop_target
            && (self.drop_candidate || self.drop_target_active)
        {
            return Some(WidgetOutput::typed(FolderTreeHitMessage::HoverDropTarget(
                position,
            )));
        }
        None
    }

    fn handle_primary_release(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        if self.drag_active && !self.drag_source && bounds.contains(position) {
            self.common.state.pressed = false;
            self.common.state.hovered = true;
            self.dragged = false;
            return Some(WidgetOutput::typed(FolderTreeHitMessage::Drop));
        }
        let activated = self.common.state.pressed && !self.dragged && bounds.contains(position);
        let dragged =
            self.drag_source || (self.common.state.pressed && (self.dragged || self.drag_active));
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

    fn paint_label(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect, theme: &ThemeTokens) {
        let font_size = if bounds.height() >= 38.0 {
            18.0
        } else if bounds.height() >= 28.0 {
            14.0
        } else {
            13.0
        };
        let label_rect = Rect::from_min_max(
            Point::new(bounds.min.x + 4.0, bounds.min.y),
            Point::new(bounds.max.x - 4.0, bounds.max.y),
        );
        let highlighted =
            self.drop_target || (self.common.state.hovered && self.drop_candidate) || self.selected;
        let color = if highlighted {
            Rgba8 {
                r: 255,
                g: 238,
                b: 224,
                a: 255,
            }
        } else {
            theme.text_primary
        };
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: self.label.clone(),
            rect: label_rect,
            font_size,
            baseline: Some((label_rect.height() * 0.5 + font_size * 0.35).max(0.0)),
            color,
            align: PaintTextAlign::Left,
            wrap: radiant::widgets::TextWrap::None,
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
        let mut first = FolderTreeHitTarget::new("kicks", false, false, false, false, false, false);
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

        let mut refreshed =
            FolderTreeHitTarget::new("kicks", false, false, true, true, false, false);
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
        let mut refreshed =
            FolderTreeHitTarget::new("kicks", false, false, true, true, false, false);
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

    #[test]
    fn active_drag_source_does_not_depend_on_retained_pressed_state() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut refreshed =
            FolderTreeHitTarget::new("kicks", false, false, true, true, false, false);

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

    #[test]
    fn active_drag_release_on_target_row_emits_drop_without_press_capture() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut target = FolderTreeHitTarget::new("loops", false, true, true, false, true, true);

        assert_eq!(
            message_from(target.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(90.0, 9.0),
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                },
            )),
            FolderTreeHitMessage::Drop
        );
    }

    #[test]
    fn drop_target_paints_highlighted_label_text() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let target = FolderTreeHitTarget::new("loops", false, true, true, false, true, true);
        let theme = ThemeTokens::default();
        let mut primitives = Vec::new();

        target.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

        assert!(
            primitives.iter().any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::Text(run)
                        if run.text == "loops" && run.color != theme.text_primary
                )
            }),
            "folder drop targets should light up the label itself, not only the row marker"
        );
    }

    #[test]
    fn drag_hover_reports_new_drop_target_once() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut target = FolderTreeHitTarget::new("loops", false, false, true, false, true, false);

        assert_eq!(
            message_from(target.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(40.0, 9.0),
                },
            )),
            FolderTreeHitMessage::HoverDropTarget(Point::new(40.0, 9.0)),
            "a new valid target must notify the app so the committed drop target can change"
        );
    }

    #[test]
    fn current_drop_target_hover_stays_local() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut target = FolderTreeHitTarget::new("loops", false, true, true, false, true, true);

        assert!(
            target
                .handle_input(
                    bounds,
                    WidgetInput::PointerMove {
                        position: Point::new(40.0, 9.0),
                    },
                )
                .is_none(),
            "pointer motion inside the already-highlighted target should not force another scene rebuild"
        );
    }

    #[test]
    fn invalid_drag_hover_only_reports_when_it_can_clear_existing_target() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut quiet_invalid =
            FolderTreeHitTarget::new("kicks", false, false, true, false, false, false);
        assert!(
            quiet_invalid
                .handle_input(
                    bounds,
                    WidgetInput::PointerMove {
                        position: Point::new(40.0, 9.0),
                    },
                )
                .is_none()
        );

        let mut clearing_invalid =
            FolderTreeHitTarget::new("kicks", false, false, true, false, false, true);
        assert_eq!(
            message_from(clearing_invalid.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(40.0, 9.0),
                },
            )),
            FolderTreeHitMessage::HoverDropTarget(Point::new(40.0, 9.0)),
            "invalid rows only need to notify the app when they can clear a previous drop target"
        );
    }
}
