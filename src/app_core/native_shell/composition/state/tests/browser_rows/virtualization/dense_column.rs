use super::*;

#[test]
fn browser_virtualization_keeps_focused_row_visible_in_dense_column() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    for visible_row in 0..200 {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:03}"),
            1,
            false,
            visible_row == 150,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(150);
    model.browser.anchor_visible_row = Some(148);
    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(!rendered.is_empty());
    assert!(rendered.iter().any(|row| row.visible_row == 150));
    assert!(rendered.first().is_some_and(|first| first.visible_row > 0));
}

#[test]
fn browser_virtualization_clamps_tail_without_dropping_last_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = browser_model_with_rows(1000, 999);
    model.browser.selected_visible_row = Some(999);
    model.browser.anchor_visible_row = Some(996);

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let expected_len = browser_rows_capacity(layout.browser_rows, style.sizing)
        .min(model.browser.rows.len())
        .max(1);
    assert_eq!(rendered.len(), expected_len);
    assert_eq!(rendered.last().map(|row| row.visible_row), Some(999));
    assert!(rendered.iter().any(|row| row.visible_row == 999));
}

#[test]
fn browser_virtualization_5k_rows_keeps_focus_and_tail_visible() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = browser_model_with_rows(5000, 4999);
    model.browser.selected_visible_row = Some(4999);
    model.browser.anchor_visible_row = Some(4995);

    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(!rendered.is_empty());
    assert_eq!(rendered.last().map(|row| row.visible_row), Some(4999));
    assert!(
        rendered
            .iter()
            .any(|row| row.visible_row == 4999 && row.focused)
    );
}
