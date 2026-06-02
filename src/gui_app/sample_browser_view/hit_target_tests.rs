use super::*;
use radiant::runtime::SurfacePaintPlan;
use radiant::widgets::{PointerButton, PointerModifiers};

/// Extracts the sample-file hit-target message from a widget output.
fn message_from(output: Option<WidgetOutput>) -> SampleFileHitMessage {
    output
        .expect("expected widget output")
        .typed_copied::<SampleFileHitMessage>()
        .expect("expected sample file message")
}

/// Reports whether the paint plan contains the row hover fill.
fn paints_hover_fill(plan: &SurfacePaintPlan) -> bool {
    plan.fill_rects().any(|fill| fill.color == HOVER_FILL)
}

#[test]
/// Verifies retained refreshes do not duplicate runtime drag-preview motion.
fn active_drag_uses_runtime_preview_after_widget_refresh() {
    let bounds = Rect::from_size(120.0, 22.0);
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
    let bounds = Rect::from_size(120.0, 22.0);
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
    let bounds = Rect::from_size(120.0, 22.0);
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

    let plan = target.paint_plan_with_defaults(bounds);
    assert!(
        !paints_hover_fill(&plan),
        "non-source rows should not paint hover highlights during active file drags"
    );
}

#[test]
/// Verifies ordinary sample rows do not request stable pointer-motion routing.
fn idle_rows_do_not_request_stable_pointer_moves() {
    let target = SampleFileHitTarget::new(false, false, false, false, false);

    assert!(
        !target.accepts_pointer_move(),
        "idle sample rows should update hover on enter/leave without routing every stable move"
    );
}

#[test]
/// Verifies pressed sample rows keep motion so drags can start reliably.
fn pressed_rows_request_pointer_moves_for_drag_start() {
    let mut target = SampleFileHitTarget::new(false, false, false, false, false);
    target.row.common.state.pressed = true;

    assert!(
        target.accepts_pointer_move(),
        "pressed sample rows need pointer motion to detect drag start"
    );
}

#[test]
/// Verifies active sample drags keep motion routing for source and non-source rows.
fn active_drag_rows_request_pointer_moves() {
    let source = SampleFileHitTarget::new(false, true, true, false, false);
    let non_source = SampleFileHitTarget::new(false, true, false, false, false);

    assert!(
        source.accepts_pointer_move(),
        "active drag source rows still need release and drag-session motion routing"
    );
    assert!(
        non_source.accepts_pointer_move(),
        "non-source rows need drag-active motion routing to clear stale hover"
    );
}

#[test]
/// Verifies stale hover state is not retained across widget refreshes.
fn hover_state_clears_on_retained_widget_refresh() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));
    assert!(previous.row.common.state.hovered);

    let mut refreshed = SampleFileHitTarget::new(false, false, false, false, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(
        !refreshed.row.common.state.hovered,
        "sample row hover paint must not stick after retained projections"
    );
    let plan = refreshed.paint_plan_with_defaults(bounds);
    assert!(
        !paints_hover_fill(&plan),
        "refreshed rows should not paint stale hover highlights"
    );
}

#[test]
/// Verifies unselected hover paint remains visually neutral.
fn hover_fill_is_neutral_not_selection_red() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut target = SampleFileHitTarget::new(false, false, false, false, false);
    target.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));

    let plan = target.paint_plan_with_defaults(bounds);

    assert!(paints_hover_fill(&plan));
    assert!(!plan.fill_rects().any(|fill| fill.color
        == Rgba8 {
            r: 255,
            g: 82,
            b: 62,
            a: 120
        }));
}

#[test]
/// Verifies row activation preserves primary-release modifier state.
fn primary_activation_preserves_release_modifiers() {
    let bounds = Rect::from_size(120.0, 22.0);
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
    let bounds = Rect::from_size(120.0, 22.0);
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
    let bounds = Rect::from_size(120.0, 22.0);
    let mut previous = SampleFileHitTarget::new(false, false, false, false, false);
    previous.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));
    assert!(previous.row.common.state.hovered);

    let mut suppressed = SampleFileHitTarget::new(false, false, false, false, true);
    suppressed.synchronize_from_previous(&previous);
    assert!(!suppressed.row.common.state.hovered);
    suppressed.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));
    assert!(!suppressed.row.common.state.hovered);

    let plan = suppressed.paint_plan_with_defaults(bounds);
    assert!(
        !paints_hover_fill(&plan),
        "suppressed rows should not paint hover highlights during sidebar resize"
    );
}

#[test]
/// Verifies cached sample rows paint the loaded marker.
fn loaded_rows_paint_right_edge_marker() {
    let bounds = Rect::from_xy_size(10.0, 20.0, 120.0, 22.0);
    let target = SampleFileHitTarget::new(false, false, false, true, false);
    let plan = target.paint_plan_with_defaults(bounds);

    assert!(
        plan.fill_rects()
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
    let bounds = Rect::from_xy_size(10.0, 20.0, 120.0, 22.0);
    let target = SampleFileHitTarget::new(false, false, false, false, false);
    let plan = target.paint_plan_with_defaults(bounds);

    assert!(
        !plan
            .fill_rects()
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
