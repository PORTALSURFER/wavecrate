use super::*;
use radiant::gui::types::Point;
use radiant::prelude::{self as ui, IntoView};
use radiant::runtime::{SurfacePaintPlan, UiSurface};
use radiant::widgets::{
    DragHandleMessage, PointerButton, PointerModifiers, WidgetInput, WidgetOutput,
};

const TEST_INPUT_ID: u64 = 99_501;

fn message_from(surface: &UiSurface<GuiMessage>, output: Option<WidgetOutput>) -> GuiMessage {
    surface
        .dispatch_widget_output(TEST_INPUT_ID, output.expect("expected widget output"))
        .expect("expected sample file message")
}

fn sample_hit_target(
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
) -> UiSurface<GuiMessage> {
    sample_hit_target_with_model(SampleFileHitTargetModel {
        file_id: "sample.wav",
        selected,
        copy_flash: false,
        drag_active,
        drag_source,
        cached,
        missing: false,
        hit_path: String::from("sample.wav"),
        help_tooltips_enabled: false,
    })
}

fn sample_hit_target_with_copy_flash(selected: bool, cached: bool) -> UiSurface<GuiMessage> {
    sample_hit_target_with_model(SampleFileHitTargetModel {
        file_id: "sample.wav",
        selected,
        copy_flash: true,
        drag_active: false,
        drag_source: false,
        cached,
        missing: false,
        hit_path: String::from("sample.wav"),
        help_tooltips_enabled: false,
    })
}

fn sample_hit_target_with_model(model: SampleFileHitTargetModel<'_>) -> UiSurface<GuiMessage> {
    sample_file_hit_target_for_tests(ui::empty(), model, TEST_INPUT_ID)
        .size(120.0, 22.0)
        .into_surface()
}

fn dispatch(
    surface: &mut UiSurface<GuiMessage>,
    bounds: ui::Rect,
    input: WidgetInput,
) -> Option<WidgetOutput> {
    surface.dispatch_widget_input(TEST_INPUT_ID, bounds, input)
}

fn synchronize_widget_from_previous(
    current: &mut UiSurface<GuiMessage>,
    previous: &UiSurface<GuiMessage>,
) {
    let previous = previous
        .find_widget(TEST_INPUT_ID)
        .expect("previous sample row widget")
        .widget();
    let current = current
        .find_widget_mut(TEST_INPUT_ID)
        .expect("current sample row widget")
        .widget_mut();
    current.synchronize_from_previous(previous);
}

fn sample_widget_plan(surface: &UiSurface<GuiMessage>, bounds: ui::Rect) -> SurfacePaintPlan {
    surface
        .find_widget(TEST_INPUT_ID)
        .expect("sample row widget")
        .widget()
        .paint_plan_with_defaults(bounds)
}

fn sample_widget_is_hovered(surface: &UiSurface<GuiMessage>) -> bool {
    surface
        .find_widget(TEST_INPUT_ID)
        .expect("sample row widget")
        .widget()
        .common()
        .is_hovered()
}

fn sample_widget_is_pressed(surface: &UiSurface<GuiMessage>) -> bool {
    surface
        .find_widget(TEST_INPUT_ID)
        .expect("sample row widget")
        .widget()
        .common()
        .is_pressed()
}

fn sample_widget_accepts_pointer_move(surface: &UiSurface<GuiMessage>) -> bool {
    surface
        .find_widget(TEST_INPUT_ID)
        .expect("sample row widget")
        .widget()
        .accepts_pointer_move()
}

fn set_sample_widget_hovered(surface: &mut UiSurface<GuiMessage>, hovered: bool) {
    surface
        .find_widget_mut(TEST_INPUT_ID)
        .expect("sample row widget")
        .widget_mut()
        .common_mut()
        .state
        .hovered = hovered;
}

fn paints_hover_fill(plan: &SurfacePaintPlan) -> bool {
    let hover = sample_row_palette_for_tests()
        .hovered
        .expect("dense-row hover fill");
    plan.fill_rects().any(|fill| fill.color == hover)
}

fn selected_fill() -> ui::Rgba8 {
    sample_row_palette_for_tests()
        .selected
        .expect("dense-row selected fill")
}

#[test]
/// Verifies copied file rows flash without pretending to be selected.
fn copied_rows_paint_flash_fill_without_selection_marker() {
    let bounds = ui::Rect::from_xy_size(10.0, 20.0, 120.0, 22.0);
    let target = sample_hit_target_with_copy_flash(false, true);
    let plan = sample_widget_plan(&target, bounds);

    assert!(
        plan.fill_rects().any(|fill| fill.color == COPY_FLASH_FILL),
        "copied rows should paint the transient copy flash fill"
    );
    assert!(
        !plan
            .fill_rects()
            .any(|fill| fill.rect.min.x == bounds.min.x && fill.color == SELECTED_MARKER),
        "copy flash must not add the selection marker"
    );
    assert!(
        !plan.fill_rects().any(|fill| fill.color == CACHED_MARKER),
        "copy flash fill should not be interrupted by the cached marker"
    );
}

#[test]
/// Verifies retained refreshes do not duplicate runtime drag-preview motion.
fn active_drag_uses_runtime_preview_after_widget_refresh() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut first = sample_hit_target(false, false, false, false);
    dispatch(
        &mut first,
        bounds,
        WidgetInput::primary_press(Point::new(6.0, 6.0)),
    );
    let output = dispatch(
        &mut first,
        bounds,
        WidgetInput::pointer_move(Point::new(16.0, 7.0)),
    );
    assert_eq!(
        message_from(&first, output),
        GuiMessage::DragSampleFile {
            path: String::from("sample.wav"),
            drag: DragHandleMessage::started(Point::new(16.0, 7.0)),
        }
    );

    let mut refreshed = sample_hit_target(false, true, true, false);
    synchronize_widget_from_previous(&mut refreshed, &first);
    assert!(
        dispatch(
            &mut refreshed,
            bounds,
            WidgetInput::pointer_move(Point::new(34.0, 8.0)),
        )
        .is_none(),
        "runtime drag preview already tracks pointer movement without app refresh"
    );
}

#[test]
/// Verifies refreshed drag-source rows can still end the drag sequence.
fn active_drag_source_does_not_depend_on_retained_pressed_state() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut refreshed = sample_hit_target(false, true, true, false);

    assert!(
        dispatch(
            &mut refreshed,
            bounds,
            WidgetInput::pointer_move(Point::new(34.0, 8.0)),
        )
        .is_none(),
        "active drag moves are runtime-local and should not require retained pressed state"
    );
    let output = dispatch(
        &mut refreshed,
        bounds,
        WidgetInput::primary_release(Point::new(220.0, 90.0)),
    );
    assert_eq!(
        message_from(&refreshed, output),
        GuiMessage::DragSampleFile {
            path: String::from("sample.wav"),
            drag: DragHandleMessage::ended(Point::new(220.0, 90.0)),
        }
    );
}

#[test]
/// Verifies non-source rows clear hover while another sample row is dragged.
fn active_drag_non_source_rows_do_not_keep_hover_highlight() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, true, false, false);
    set_sample_widget_hovered(&mut target, true);

    assert!(
        dispatch(
            &mut target,
            bounds,
            WidgetInput::pointer_move(Point::new(34.0, 8.0)),
        )
        .is_none()
    );
    assert!(
        !sample_widget_is_hovered(&target),
        "sample rows should not retain hover while another file is being dragged"
    );

    let plan = sample_widget_plan(&target, bounds);
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
        !sample_widget_accepts_pointer_move(&target),
        "idle sample rows should update hover on enter/leave without routing every stable move"
    );
}

#[test]
/// Verifies pressed sample rows keep motion so drags can start reliably.
fn pressed_rows_request_pointer_moves_for_drag_start() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, false, false, false);
    dispatch(
        &mut target,
        bounds,
        WidgetInput::primary_press(Point::new(34.0, 8.0)),
    );

    assert!(
        sample_widget_accepts_pointer_move(&target),
        "pressed sample rows need pointer motion to detect drag start"
    );
}

#[test]
/// Verifies active sample drags keep motion routing for source and non-source rows.
fn active_drag_rows_request_pointer_moves() {
    let source = sample_hit_target(false, true, true, false);
    let non_source = sample_hit_target(false, true, false, false);

    assert!(
        sample_widget_accepts_pointer_move(&source),
        "active drag source rows still need release and drag-session motion routing"
    );
    assert!(
        sample_widget_accepts_pointer_move(&non_source),
        "non-source rows need drag-active motion routing to clear stale hover"
    );
}

#[test]
/// Verifies retained refreshes keep hover for the current row owner.
fn hover_state_survives_retained_widget_refresh() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut previous = sample_hit_target(false, false, false, false);
    dispatch(
        &mut previous,
        bounds,
        WidgetInput::pointer_move(Point::new(34.0, 8.0)),
    );
    assert!(sample_widget_is_hovered(&previous));

    let mut refreshed = sample_hit_target(false, false, false, false);
    synchronize_widget_from_previous(&mut refreshed, &previous);

    assert!(
        sample_widget_is_hovered(&refreshed),
        "sample row hover paint must survive retained projections for the current hover owner"
    );
    let plan = sample_widget_plan(&refreshed, bounds);
    assert!(
        paints_hover_fill(&plan),
        "refreshed current-hover rows should keep painting hover highlights"
    );
}

#[test]
/// Verifies unselected hover paint remains visually neutral.
fn hover_fill_is_neutral_not_selection_red() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, false, false, false);
    dispatch(
        &mut target,
        bounds,
        WidgetInput::pointer_move(Point::new(34.0, 8.0)),
    );

    let plan = sample_widget_plan(&target, bounds);

    assert!(paints_hover_fill(&plan));
    assert!(!plan.fill_rects().any(|fill| fill.color == selected_fill()));
}

#[test]
/// Verifies row activation preserves primary-release modifier state.
fn primary_activation_preserves_release_modifiers() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, false, false, false);
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    dispatch(
        &mut target,
        bounds,
        WidgetInput::primary_press(Point::new(34.0, 8.0)),
    );

    let output = dispatch(
        &mut target,
        bounds,
        WidgetInput::pointer_release(Point::new(34.0, 8.0), PointerButton::Primary, modifiers),
    );
    assert_eq!(
        message_from(&target, output),
        GuiMessage::SelectSampleWithModifiers {
            path: String::from("sample.wav"),
            modifiers,
        }
    );
}

#[test]
/// Verifies sample-row double activation uses the normal activation action.
fn double_activation_uses_normal_sample_activation() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut target = sample_hit_target(false, false, false, false);

    let output = dispatch(
        &mut target,
        bounds,
        WidgetInput::primary_double_click(Point::new(34.0, 8.0)),
    );
    assert_eq!(
        message_from(&target, output),
        GuiMessage::SelectSampleWithModifiers {
            path: String::from("sample.wav"),
            modifiers: PointerModifiers::default(),
        }
    );
}

#[test]
/// Verifies retained pressed and hover state survive a row refresh.
fn pressed_and_hover_state_survive_retained_widget_refresh() {
    let bounds = ui::Rect::from_size(120.0, 22.0);
    let mut previous = sample_hit_target(false, false, false, false);
    dispatch(
        &mut previous,
        bounds,
        WidgetInput::primary_press(Point::new(34.0, 8.0)),
    );
    assert!(sample_widget_is_hovered(&previous));
    assert!(sample_widget_is_pressed(&previous));

    let mut refreshed = sample_hit_target(false, false, false, false);
    synchronize_widget_from_previous(&mut refreshed, &previous);

    assert!(sample_widget_is_hovered(&refreshed));
    assert!(sample_widget_is_pressed(&refreshed));
}

#[test]
/// Verifies cached sample rows paint the loaded marker.
fn loaded_rows_paint_right_edge_marker() {
    let bounds = ui::Rect::from_xy_size(10.0, 20.0, 120.0, 22.0);
    let target = sample_hit_target(false, false, false, true);
    let plan = sample_widget_plan(&target, bounds);

    assert!(
        plan.fill_rects()
            .any(|fill| fill.rect.min.x == bounds.max.x - 3.0
                && fill.rect.width() == 2.0
                && fill.color == CACHED_MARKER),
        "loaded rows should show a near-white right-edge marker"
    );
}

#[test]
/// Verifies uncached sample rows do not paint the loaded marker.
fn unloaded_rows_do_not_paint_loaded_marker() {
    let bounds = ui::Rect::from_xy_size(10.0, 20.0, 120.0, 22.0);
    let target = sample_hit_target(false, false, false, false);
    let plan = sample_widget_plan(&target, bounds);

    assert!(
        !plan
            .fill_rects()
            .any(|fill| fill.rect.min.x == bounds.max.x - 3.0
                && fill.rect.width() == 2.0
                && fill.color == CACHED_MARKER),
        "unloaded rows should not show the loaded marker"
    );
}
