use super::*;
use radiant::gui::types::Point;
use radiant::runtime::SurfacePaintPlan;
use radiant::widgets::{
    DragHandleMessage, PointerButton, PointerModifiers, Widget, WidgetInput, WidgetOutput,
};

/// Extracts the sample-file hit-target message from a widget output.
fn message_from(output: Option<WidgetOutput>) -> GuiMessage {
    output
        .expect("expected widget output")
        .typed_cloned::<GuiMessage>()
        .expect("expected sample file message")
}

fn sample_hit_target(
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
) -> SampleFileHitTarget {
    SampleFileHitTarget::new(
        String::from("sample.wav"),
        selected,
        false,
        drag_active,
        drag_source,
        cached,
        false,
    )
}

fn sample_hit_target_with_copy_flash(selected: bool, cached: bool) -> SampleFileHitTarget {
    SampleFileHitTarget::new(
        String::from("sample.wav"),
        selected,
        true,
        false,
        false,
        cached,
        false,
    )
}

/// Reports whether the paint plan contains the row hover fill.
fn paints_hover_fill(plan: &SurfacePaintPlan) -> bool {
    let hover = sample_row_palette_for_tests()
        .hovered
        .expect("dense-row hover fill");
    plan.fill_rects().any(|fill| fill.color == hover)
}

fn selected_marker_color() -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 82,
        b: 62,
        a: 245,
    }
}

fn cached_marker_color() -> Rgba8 {
    Rgba8 {
        r: 226,
        g: 226,
        b: 226,
        a: 210,
    }
}

fn selected_fill() -> Rgba8 {
    sample_row_palette_for_tests()
        .selected
        .expect("dense-row selected fill")
}

fn is_hovered(target: &SampleFileHitTarget) -> bool {
    target.common().is_hovered()
}

fn is_pressed(target: &SampleFileHitTarget) -> bool {
    target.common().is_pressed()
}

#[test]
/// Verifies copied file rows flash without pretending to be selected.
fn copied_rows_paint_flash_fill_without_selection_marker() {
    let bounds = Rect::from_xy_size(10.0, 20.0, 120.0, 22.0);
    let target = sample_hit_target_with_copy_flash(false, true);
    let plan = target.paint_plan_with_defaults(bounds);

    assert!(
        plan.fill_rects().any(|fill| fill.color == COPY_FLASH_FILL),
        "copied rows should paint the transient copy flash fill"
    );
    assert!(
        !plan
            .fill_rects()
            .any(|fill| fill.rect.min.x == bounds.min.x && fill.color == selected_marker_color()),
        "copy flash must not add the selection marker"
    );
    assert!(
        !plan
            .fill_rects()
            .any(|fill| fill.color == cached_marker_color()),
        "copy flash fill should not be interrupted by the cached marker"
    );
}

#[test]
/// Verifies retained refreshes do not duplicate runtime drag-preview motion.
fn active_drag_uses_runtime_preview_after_widget_refresh() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut first = sample_hit_target(false, false, false, false);
    first.handle_input(bounds, WidgetInput::primary_press(Point::new(6.0, 6.0)));
    assert_eq!(
        message_from(first.handle_input(bounds, WidgetInput::pointer_move(Point::new(16.0, 7.0)),)),
        GuiMessage::DragSampleFile {
            path: String::from("sample.wav"),
            drag: DragHandleMessage::started(Point::new(16.0, 7.0)),
        }
    );

    let mut refreshed = sample_hit_target(false, true, true, false);
    refreshed.synchronize_from_previous(&first);
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
    let mut refreshed = sample_hit_target(false, true, true, false);

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
        GuiMessage::DragSampleFile {
            path: String::from("sample.wav"),
            drag: DragHandleMessage::ended(Point::new(220.0, 90.0)),
        }
    );
}

#[test]
/// Verifies non-source rows clear hover while another sample row is dragged.
fn active_drag_non_source_rows_do_not_keep_hover_highlight() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, true, false, false);
    target.common_mut().state.hovered = true;

    assert!(
        target
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)),)
            .is_none()
    );
    assert!(
        !is_hovered(&target),
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
    let target = sample_hit_target(false, false, false, false);

    assert!(
        !target.accepts_pointer_move(),
        "idle sample rows should update hover on enter/leave without routing every stable move"
    );
}

#[test]
/// Verifies pressed sample rows keep motion so drags can start reliably.
fn pressed_rows_request_pointer_moves_for_drag_start() {
    let mut target = sample_hit_target(false, false, false, false);
    target.handle_input(
        Rect::from_size(120.0, 22.0),
        WidgetInput::primary_press(Point::new(34.0, 8.0)),
    );

    assert!(
        target.accepts_pointer_move(),
        "pressed sample rows need pointer motion to detect drag start"
    );
}

#[test]
/// Verifies active sample drags keep motion routing for source and non-source rows.
fn active_drag_rows_request_pointer_moves() {
    let source = sample_hit_target(false, true, true, false);
    let non_source = sample_hit_target(false, true, false, false);

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
/// Verifies retained refreshes keep hover for the current row owner.
fn hover_state_survives_retained_widget_refresh() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut previous = sample_hit_target(false, false, false, false);
    previous.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));
    assert!(is_hovered(&previous));

    let mut refreshed = sample_hit_target(false, false, false, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(
        is_hovered(&refreshed),
        "sample row hover paint must survive retained projections for the current hover owner"
    );
    let plan = refreshed.paint_plan_with_defaults(bounds);
    assert!(
        paints_hover_fill(&plan),
        "refreshed current-hover rows should keep painting hover highlights"
    );
}

#[test]
/// Verifies unselected hover paint remains visually neutral.
fn hover_fill_is_neutral_not_selection_red() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, false, false, false);
    target.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)));

    let plan = target.paint_plan_with_defaults(bounds);

    assert!(paints_hover_fill(&plan));
    assert!(!plan.fill_rects().any(|fill| fill.color == selected_fill()));
}

#[test]
/// Verifies row activation preserves primary-release modifier state.
fn primary_activation_preserves_release_modifiers() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, false, false, false);
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
        GuiMessage::SelectSampleWithModifiers {
            path: String::from("sample.wav"),
            modifiers,
        }
    );
}

#[test]
/// Verifies sample-row double activation uses the normal activation action.
fn double_activation_uses_normal_sample_activation() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, false, false, false);

    assert_eq!(
        message_from(target.handle_input(
            bounds,
            WidgetInput::primary_double_click(Point::new(34.0, 8.0))
        )),
        GuiMessage::SelectSampleWithModifiers {
            path: String::from("sample.wav"),
            modifiers: PointerModifiers::default(),
        }
    );
}

#[test]
/// Verifies retained pressed and hover state survive a row refresh.
fn pressed_and_hover_state_survive_retained_widget_refresh() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut previous = sample_hit_target(false, false, false, false);
    previous.handle_input(bounds, WidgetInput::primary_press(Point::new(34.0, 8.0)));
    assert!(is_hovered(&previous));
    assert!(is_pressed(&previous));

    let mut refreshed = sample_hit_target(false, false, false, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(is_hovered(&refreshed));
    assert!(is_pressed(&refreshed));
}

#[test]
/// Verifies cached sample rows paint the loaded marker.
fn loaded_rows_paint_right_edge_marker() {
    let bounds = Rect::from_xy_size(10.0, 20.0, 120.0, 22.0);
    let target = sample_hit_target(false, false, false, true);
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
    let target = sample_hit_target(false, false, false, false);
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
