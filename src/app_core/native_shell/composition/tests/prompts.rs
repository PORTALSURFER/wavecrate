use super::*;

fn prompt_dialog(layout: &ShellLayout, style: &style::StyleTokens) -> crate::gui::types::Rect {
    let sizing = style.sizing;
    let width = sizing
        .prompt_width
        .min(layout.content.width() - (sizing.overlay_padding * 2.0))
        .max(260.0);
    let height = sizing
        .prompt_min_height
        .min(layout.content.height() - (sizing.overlay_padding * 2.0))
        .max(108.0);
    let x = layout.content.min.x + (layout.content.width() - width).max(0.0) * 0.5;
    let y = layout.content.min.y + (layout.content.height() - height).max(0.0) * 0.35;
    crate::gui::types::Rect::from_min_max(Point::new(x, y), Point::new(x + width, y + height))
}

fn prompt_buttons(
    dialog: crate::gui::types::Rect,
    style: &style::StyleTokens,
) -> (crate::gui::types::Rect, crate::gui::types::Rect) {
    let sizing = style.sizing;
    let cancel = crate::gui::types::Rect::from_min_max(
        Point::new(
            dialog.max.x - sizing.overlay_button_width - sizing.text_inset_x,
            dialog.max.y - sizing.overlay_button_height - sizing.text_inset_y,
        ),
        Point::new(
            dialog.max.x - sizing.text_inset_x,
            dialog.max.y - sizing.text_inset_y,
        ),
    );
    let confirm = crate::gui::types::Rect::from_min_max(
        Point::new(
            cancel.min.x - sizing.overlay_button_width - sizing.action_button_gap,
            cancel.min.y,
        ),
        Point::new(cancel.min.x - sizing.action_button_gap, cancel.max.y),
    );
    (confirm, cancel)
}

#[test]
fn prompt_hit_test_emits_confirm_and_cancel() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.confirm_prompt.visible = true;
    let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
    let dialog = prompt_dialog(&layout, &style);
    let (confirm, cancel) = prompt_buttons(dialog, &style);
    let confirm_point = Point::new(
        (confirm.min.x + confirm.max.x) * 0.5,
        (confirm.min.y + confirm.max.y) * 0.5,
    );
    let cancel_point = Point::new(
        (cancel.min.x + cancel.max.x) * 0.5,
        (cancel.min.y + cancel.max.y) * 0.5,
    );
    assert_eq!(
        state.prompt_action_at_point(&layout, &model, confirm_point),
        Some(crate::compat_app_contract::UiAction::ConfirmPrompt)
    );
    assert_eq!(
        state.prompt_action_at_point(&layout, &model, cancel_point),
        Some(crate::compat_app_contract::UiAction::CancelPrompt)
    );
}

#[test]
fn prompt_input_hit_test_resolves_text_entry_rect() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.confirm_prompt.visible = true;
    model.confirm_prompt.input_value = Some(String::from("kicks"));
    let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
    let sizing = style.sizing;
    let dialog = prompt_dialog(&layout, &style);
    let input_y = dialog.min.y
        + sizing.text_inset_y
        + sizing.font_title
        + sizing.font_meta
        + (sizing.text_row_gap * 4.0);
    let point = Point::new(dialog.min.x + 20.0, input_y + 8.0);
    assert!(state.prompt_input_at_point(&layout, &model, point));
}

#[test]
fn prompt_confirm_hit_test_is_blocked_when_input_error_is_present() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.confirm_prompt.visible = true;
    model.confirm_prompt.input_value = Some(String::from("bad/name"));
    model.confirm_prompt.input_error =
        Some(String::from("Folder name cannot contain path separators"));
    let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
    let dialog = prompt_dialog(&layout, &style);
    let (confirm, _) = prompt_buttons(dialog, &style);
    let confirm_point = Point::new(
        (confirm.min.x + confirm.max.x) * 0.5,
        (confirm.min.y + confirm.max.y) * 0.5,
    );
    assert_eq!(
        state.prompt_action_at_point(&layout, &model, confirm_point),
        None
    );
}
