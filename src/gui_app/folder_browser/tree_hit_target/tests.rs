use super::*;
use radiant::widgets::PointerModifiers;

fn message_from(output: Option<WidgetOutput>) -> FolderTreeHitMessage {
    *output
        .expect("expected widget output")
        .typed_ref::<FolderTreeHitMessage>()
        .expect("expected folder tree message")
}

fn row_bounds() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0))
}

#[test]
fn active_drag_survives_widget_refresh_as_moved() {
    let bounds = row_bounds();
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

    let mut refreshed = FolderTreeHitTarget::new("kicks", false, false, true, true, false, false);
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
    let bounds = row_bounds();
    let mut refreshed = FolderTreeHitTarget::new("kicks", false, false, true, true, false, false);
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
    let bounds = row_bounds();
    let mut refreshed = FolderTreeHitTarget::new("kicks", false, false, true, true, false, false);

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
    let bounds = row_bounds();
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
    let bounds = row_bounds();
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
    let bounds = row_bounds();
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
    let bounds = row_bounds();
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
    let bounds = row_bounds();
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
