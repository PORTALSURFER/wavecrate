use super::*;
use radiant::{
    gui::types::Point,
    layout::LayoutOutput,
    theme::ThemeTokens,
    widgets::{DragHandleMessage, Widget, WidgetInput, WidgetOutput},
};

fn message_from(output: Option<WidgetOutput>) -> GuiMessage {
    output
        .expect("expected widget output")
        .typed_cloned::<GuiMessage>()
        .expect("expected folder tree message")
}

fn folder_message(message: FolderBrowserMessage) -> GuiMessage {
    GuiMessage::FolderBrowser(message)
}

fn folder_hit_target(
    label: &str,
    selected: bool,
    drop_target: bool,
    drag_active: bool,
    drag_source: bool,
    drop_candidate: bool,
    drop_target_active: bool,
) -> FolderTreeHitTarget {
    FolderTreeHitTarget::new(
        label.to_string(),
        label,
        selected,
        drop_target,
        drag_active,
        drag_source,
        drop_candidate,
        drop_target_active,
    )
}

fn row_bounds() -> Rect {
    Rect::from_size(120.0, 22.0)
}

fn is_hovered(target: &FolderTreeHitTarget) -> bool {
    target.common().is_hovered()
}

#[test]
fn active_drag_survives_widget_refresh_as_moved() {
    let bounds = row_bounds();
    let mut first = folder_hit_target("kicks", false, false, false, false, false, false);
    first.handle_input(bounds, WidgetInput::primary_press(Point::new(6.0, 6.0)));
    assert_eq!(
        message_from(first.handle_input(bounds, WidgetInput::pointer_move(Point::new(16.0, 7.0)),)),
        folder_message(FolderBrowserMessage::DragFolder(
            String::from("kicks"),
            DragHandleMessage::started(Point::new(16.0, 7.0)),
        ))
    );

    let mut refreshed = folder_hit_target("kicks", false, false, true, true, false, false);
    refreshed.synchronize_from_previous(&first);
    assert_eq!(
        message_from(
            refreshed.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)),)
        ),
        folder_message(FolderBrowserMessage::DragFolder(
            String::from("kicks"),
            DragHandleMessage::moved(Point::new(34.0, 8.0)),
        ))
    );
}

#[test]
fn active_drag_survives_widget_refresh_until_release() {
    let bounds = row_bounds();
    let mut refreshed = folder_hit_target("kicks", false, false, true, true, false, false);
    refreshed.handle_input(bounds, WidgetInput::primary_press(Point::new(6.0, 6.0)));

    assert_eq!(
        message_from(
            refreshed.handle_input(bounds, WidgetInput::primary_release(Point::new(90.0, 9.0)),)
        ),
        folder_message(FolderBrowserMessage::DragFolder(
            String::from("kicks"),
            DragHandleMessage::ended(Point::new(90.0, 9.0)),
        ))
    );
}

#[test]
fn active_drag_source_does_not_depend_on_retained_pressed_state() {
    let bounds = row_bounds();
    let mut refreshed = folder_hit_target("kicks", false, false, true, true, false, false);

    assert_eq!(
        message_from(
            refreshed.handle_input(bounds, WidgetInput::pointer_move(Point::new(34.0, 8.0)),)
        ),
        folder_message(FolderBrowserMessage::DragFolder(
            String::from("kicks"),
            DragHandleMessage::moved(Point::new(34.0, 8.0)),
        ))
    );
    assert_eq!(
        message_from(
            refreshed.handle_input(bounds, WidgetInput::primary_release(Point::new(90.0, 9.0)),)
        ),
        folder_message(FolderBrowserMessage::DragFolder(
            String::from("kicks"),
            DragHandleMessage::ended(Point::new(90.0, 9.0)),
        ))
    );
}

#[test]
fn active_drag_release_on_target_row_emits_drop_without_press_capture() {
    let bounds = row_bounds();
    let mut target = folder_hit_target("loops", false, true, true, false, true, true);

    assert_eq!(
        message_from(
            target.handle_input(bounds, WidgetInput::primary_release(Point::new(90.0, 9.0)),)
        ),
        folder_message(FolderBrowserMessage::DropOnFolder(String::from("loops")))
    );
}

#[test]
fn double_activation_uses_normal_folder_activation() {
    let bounds = row_bounds();
    let mut target = folder_hit_target("kicks", false, false, false, false, false, false);

    assert_eq!(
        message_from(target.handle_input(
            bounds,
            WidgetInput::primary_double_click(Point::new(12.0, 9.0))
        )),
        folder_message(FolderBrowserMessage::ActivateFolder(String::from("kicks")))
    );
}

#[test]
fn drop_target_paints_highlighted_label_text() {
    let bounds = row_bounds();
    let target = folder_hit_target("loops", false, true, true, false, true, true);
    let theme = ThemeTokens::default();
    let plan = target.paint_plan(bounds, &LayoutOutput::default(), &theme);

    assert!(
        plan.text_runs()
            .any(|run| run.text == "loops" && run.color != theme.text_primary),
        "folder drop targets should light up the label itself, not only the row marker"
    );
}

#[test]
fn drag_hover_reports_new_drop_target_once() {
    let bounds = row_bounds();
    let mut target = folder_hit_target("loops", false, false, true, false, true, false);

    assert_eq!(
        message_from(
            target.handle_input(bounds, WidgetInput::pointer_move(Point::new(40.0, 9.0)),)
        ),
        folder_message(FolderBrowserMessage::HoverDropTarget(
            String::from("loops"),
            Point::new(40.0, 9.0),
        )),
        "a new valid target must notify the app so the committed drop target can change"
    );
}

#[test]
fn current_drop_target_hover_stays_local() {
    let bounds = row_bounds();
    let mut target = folder_hit_target("loops", false, true, true, false, true, true);

    assert!(
        target
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(40.0, 9.0)),)
            .is_none(),
        "pointer motion inside the already-highlighted target should not force another scene rebuild"
    );
}

#[test]
fn drag_candidate_refresh_clears_retained_hover() {
    let mut previous = folder_hit_target("loops", false, false, false, false, false, false);
    previous.handle_input(
        row_bounds(),
        WidgetInput::pointer_move(Point::new(40.0, 9.0)),
    );
    assert!(is_hovered(&previous));

    let mut refreshed = folder_hit_target("loops", false, false, true, false, true, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(
        !is_hovered(&refreshed),
        "drag-candidate rows must not inherit stale hover paint from the pre-drag tree"
    );
}

#[test]
fn active_drop_target_refresh_preserves_retained_hover() {
    let mut previous = folder_hit_target("loops", false, true, true, false, true, true);
    previous.handle_input(
        row_bounds(),
        WidgetInput::pointer_move(Point::new(40.0, 9.0)),
    );
    assert!(is_hovered(&previous));

    let mut refreshed = folder_hit_target("loops", false, true, true, false, true, true);
    refreshed.synchronize_from_previous(&previous);

    assert!(
        is_hovered(&refreshed),
        "active folder drop targets must keep hover feedback across drag-state refreshes"
    );
}

#[test]
fn invalid_drag_hover_only_reports_when_it_can_clear_existing_target() {
    let bounds = row_bounds();
    let mut quiet_invalid = folder_hit_target("kicks", false, false, true, false, false, false);
    assert!(
        quiet_invalid
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(40.0, 9.0)),)
            .is_none()
    );

    let mut clearing_invalid = folder_hit_target("kicks", false, false, true, false, false, true);
    assert_eq!(
        message_from(
            clearing_invalid
                .handle_input(bounds, WidgetInput::pointer_move(Point::new(40.0, 9.0)),)
        ),
        folder_message(FolderBrowserMessage::ClearDropTargetUnless(
            String::from("kicks"),
            Point::new(40.0, 9.0),
        )),
        "invalid rows only need to notify the app when they can clear a previous drop target"
    );
}

#[test]
fn normal_folder_hover_does_not_request_stable_pointer_moves() {
    let target = folder_hit_target("kicks", false, false, false, false, false, false);

    assert!(
        !target.accepts_pointer_move(),
        "ordinary folder hover should update on enter and leave without rerouting every stable move"
    );
}

#[test]
fn drag_folder_hover_keeps_stable_pointer_moves_for_drop_feedback() {
    let target = folder_hit_target("loops", false, false, true, false, true, true);

    assert!(
        target.accepts_pointer_move(),
        "folder drop feedback still needs stable pointer moves while an item is being dragged"
    );
}

#[test]
fn pressed_folder_row_keeps_stable_pointer_moves_for_drag_start() {
    let mut target = folder_hit_target("kicks", false, false, false, false, false, false);
    target.handle_input(
        row_bounds(),
        WidgetInput::primary_press(Point::new(6.0, 6.0)),
    );

    assert!(
        target.accepts_pointer_move(),
        "pressed rows must keep motion so a drag can start without depending on hover churn"
    );
}

#[test]
fn folder_hover_state_survives_surface_refresh() {
    let mut previous = folder_hit_target("kicks", false, false, false, false, false, false);
    previous.handle_input(
        row_bounds(),
        WidgetInput::pointer_move(Point::new(6.0, 6.0)),
    );

    let mut refreshed = folder_hit_target("kicks", false, false, false, false, false, false);
    refreshed.synchronize_from_previous(&previous);

    assert!(
        is_hovered(&refreshed),
        "surface refreshes must not clear custom folder-row hover while the runtime still owns that hover"
    );
}
