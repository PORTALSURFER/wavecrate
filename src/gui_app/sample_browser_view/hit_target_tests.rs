use super::*;
use radiant::widgets::{PointerButton, PointerModifiers};

/// Extracts the sample-file hit-target message from a widget output.
fn message_from(output: Option<WidgetOutput>) -> SampleFileHitMessage {
    *output
        .expect("expected widget output")
        .typed_ref::<SampleFileHitMessage>()
        .expect("expected sample file message")
}

/// Reports whether the paint plan contains the row hover fill.
fn paints_hover_fill(primitives: &[PaintPrimitive]) -> bool {
    primitives
        .iter()
        .filter_map(PaintPrimitive::fill_rect)
        .any(|fill| fill.color == HOVER_FILL)
}

#[test]
/// Verifies retained refreshes do not duplicate runtime drag-preview motion.
fn active_drag_uses_runtime_preview_after_widget_refresh() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut first = SampleFileHitTarget::new(false, false, false, false, false);
    first.handle_input(bounds, WidgetInput::primary_press(Point::new(6.0, 6.0)));
    assert_eq!(
        message_from(first.handle_input(bounds, WidgetInput::pointer_move(Point::new(16.0, 7.0)),)),
        SampleFileHitMessage::Drag(DragHandleMessage::Started {
            position: Point::new(16.0, 7.0),
        })
    );

    let mut refreshed = SampleFileHitTarget::new(false, true, true, false, false);
    refreshed.row.common.state = first.row.common.state;
    assert!(
        refreshed
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)),)
            .is_none(),
        "runtime drag preview already tracks pointer movement without app refresh"
    );
}

#[test]
/// Verifies refreshed drag-source rows can still end the drag sequence.
fn active_drag_source_does_not_depend_on_retained_pressed_state() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut refreshed = SampleFileHitTarget::new(false, true, true, false, false);

    assert!(
        refreshed
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)),)
            .is_none(),
        "active drag moves are runtime-local and should not require retained pressed state"
    );
    assert_eq!(
        message_from(refreshed.handle_input(
            bounds,
            WidgetInput::primary_release(Point::new(220.0, 90.0)),
        )),
        SampleFileHitMessage::Drag(DragHandleMessage::Ended {
            position: Point::new(220.0, 90.0),
        })
    );
}

#[test]
/// Verifies non-source rows clear hover while another sample row is dragged.
fn active_drag_non_source_rows_do_not_keep_hover_highlight() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut target = SampleFileHitTarget::new(false, true, false, false, false);
    target.row.common.state.hovered = true;

    assert!(
        target
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)),)
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
/// Verifies stale hover state is not retained across widget refreshes.
fn hover_state_clears_on_retained_widget_refresh() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));
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
/// Verifies unselected hover paint remains visually neutral.
fn hover_fill_is_neutral_not_selection_red() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut target = SampleFileHitTarget::new(false, false, false, false, false);
    target.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));

    let mut primitives = Vec::new();
    target.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(paints_hover_fill(&primitives));
    assert!(
        !primitives
            .iter()
            .filter_map(PaintPrimitive::fill_rect)
            .any(|fill| fill.color
                == Rgba8 {
                    r: 255,
                    g: 82,
                    b: 62,
                    a: 120
                })
    );
}

#[test]
/// Verifies row activation preserves primary-release modifier state.
fn primary_activation_preserves_release_modifiers() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut target = SampleFileHitTarget::new(false, false, false, false, false);
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    target.handle_input(bounds, WidgetInput::primary_press(Point::new(34.0, 8.0)));

    assert_eq!(
        message_from(target.handle_input(
            bounds,
            WidgetInput::pointer_release(Point::new(34.0, 8.0), PointerButton::Primary, modifiers),
        )),
        SampleFileHitMessage::Activate(modifiers)
    );
}

#[test]
/// Verifies retained pressed state survives without carrying stale hover.
fn pressed_state_survives_retained_widget_refresh_without_hover() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(bounds, WidgetInput::primary_press(Point::new(34.0, 8.0)));
    assert!(previous.row.common.state.hovered);
    assert!(previous.row.common.state.pressed);

    let mut refreshed = SampleFileHitTarget::new(false, false, false, false, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(!refreshed.row.common.state.hovered);
    assert!(refreshed.row.common.state.pressed);
}

#[test]
/// Verifies suppressed rows clear hover and omit stale hover paint.
fn suppressed_hover_clears_and_omits_stale_hover_paint() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));
    assert!(previous.row.common.state.hovered);

    let mut suppressed = SampleFileHitTarget::new(false, false, false, false, true);
    suppressed.synchronize_from_previous(&previous);
    assert!(!suppressed.row.common.state.hovered);
    suppressed.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));
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
/// Verifies cached sample rows paint the loaded marker.
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
        primitives
            .iter()
            .filter_map(PaintPrimitive::fill_rect)
            .any(|fill| fill.rect.min.x == bounds.max.x - 3.0
                && fill.rect.width() == 2.0
                && fill.color
                    == Rgba8 {
                        r: 226,
                        g: 226,
                        b: 226,
                        a: 210
                    }),
        "loaded rows should show a near-white right-edge marker"
    );
}

#[test]
/// Verifies uncached sample rows do not paint the loaded marker.
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
        !primitives
            .iter()
            .filter_map(PaintPrimitive::fill_rect)
            .any(|fill| fill.rect.min.x == bounds.max.x - 3.0
                && fill.rect.width() == 2.0
                && fill.color
                    == Rgba8 {
                        r: 226,
                        g: 226,
                        b: 226,
                        a: 210
                    }),
        "unloaded rows should not show the loaded marker"
    );
}
