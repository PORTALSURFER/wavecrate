
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
    let mut first = SampleFileHitTarget::new(false, false, false, false, false);
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

    let mut refreshed = SampleFileHitTarget::new(false, true, true, false, false);
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
    let mut refreshed = SampleFileHitTarget::new(false, true, true, false, false);

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
    let mut target = SampleFileHitTarget::new(false, true, false, false, false);
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
        !primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color.a == 155)
        ),
        "non-source rows should not paint hover highlights during active file drags"
    );
}

#[test]
fn hover_state_survives_retained_widget_refresh() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(34.0, 8.0),
        },
    );
    assert!(previous.common.state.hovered);

    let mut refreshed = SampleFileHitTarget::new(false, false, false, false, false);
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
        primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color.a == 155)
        ),
        "refreshed hovered row should keep painting the hover highlight"
    );
}

#[test]
fn suppressed_hover_clears_and_omits_stale_hover_paint() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(34.0, 8.0),
        },
    );
    assert!(previous.common.state.hovered);

    let mut suppressed = SampleFileHitTarget::new(false, false, false, false, true);
    suppressed.synchronize_from_previous(&previous);
    assert!(!suppressed.common.state.hovered);
    suppressed.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(34.0, 8.0),
        },
    );
    assert!(!suppressed.common.state.hovered);

    let mut primitives = Vec::new();
    suppressed.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        !primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color.a == 155)
        ),
        "suppressed rows should not paint hover highlights during sidebar resize"
    );
}

#[test]
fn loaded_rows_paint_right_edge_marker() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 22.0));
    let target = SampleFileHitTarget::new(false, false, false, true, false);
    let mut primitives = Vec::new();

    target.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.rect.min.x == bounds.max.x - 3.0
                        && fill.rect.width() == 2.0
                        && fill.color == Rgba8 { r: 226, g: 226, b: 226, a: 210 }
            )
        }),
        "loaded rows should show a near-white right-edge marker"
    );
}

#[test]
fn unloaded_rows_do_not_paint_loaded_marker() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 22.0));
    let target = SampleFileHitTarget::new(false, false, false, false, false);
    let mut primitives = Vec::new();

    target.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        !primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.rect.min.x == bounds.max.x - 3.0
                        && fill.rect.width() == 2.0
                        && fill.color == Rgba8 { r: 226, g: 226, b: 226, a: 210 }
            )
        }),
        "unloaded rows should not show the loaded marker"
    );
}
