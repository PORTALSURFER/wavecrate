use super::*;

fn message_from(output: Option<WidgetOutput>) -> SampleFileHitMessage {
    *output
        .expect("expected widget output")
        .typed_ref::<SampleFileHitMessage>()
        .expect("expected sample file message")
}

fn paints_hover_fill(primitives: &[PaintPrimitive]) -> bool {
    primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == HOVER_FILL),
    )
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
    refreshed.row.common.state = first.row.common.state;
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
    target.row.common.state.hovered = true;

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
        !target.row.common.state.hovered,
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
        !paints_hover_fill(&primitives),
        "non-source rows should not paint hover highlights during active file drags"
    );
}

#[test]
fn hover_state_clears_on_retained_widget_refresh() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(34.0, 8.0),
        },
    );
    assert!(previous.row.common.state.hovered);

    let mut refreshed = SampleFileHitTarget::new(false, false, false, false, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(
        !refreshed.row.common.state.hovered,
        "sample row hover paint must not stick after retained projections"
    );
    let mut primitives = Vec::new();
    refreshed.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        !paints_hover_fill(&primitives),
        "refreshed rows should not paint stale hover highlights"
    );
}

#[test]
fn hover_fill_is_neutral_not_selection_red() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut target = SampleFileHitTarget::new(false, false, false, false, false);
    target.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(34.0, 8.0),
        },
    );

    let mut primitives = Vec::new();
    target.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(paints_hover_fill(&primitives));
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        PaintPrimitive::FillRect(fill)
            if fill.color == Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120
            }
    )));
}

#[test]
fn pressed_state_survives_retained_widget_refresh_without_hover() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(34.0, 8.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(previous.row.common.state.hovered);
    assert!(previous.row.common.state.pressed);

    let mut refreshed = SampleFileHitTarget::new(false, false, false, false, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(!refreshed.row.common.state.hovered);
    assert!(refreshed.row.common.state.pressed);
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
    assert!(previous.row.common.state.hovered);

    let mut suppressed = SampleFileHitTarget::new(false, false, false, false, true);
    suppressed.synchronize_from_previous(&previous);
    assert!(!suppressed.row.common.state.hovered);
    suppressed.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(34.0, 8.0),
        },
    );
    assert!(!suppressed.row.common.state.hovered);

    let mut primitives = Vec::new();
    suppressed.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        !paints_hover_fill(&primitives),
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
