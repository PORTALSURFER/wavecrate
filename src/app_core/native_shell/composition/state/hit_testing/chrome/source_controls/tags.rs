use super::*;
use crate::app_core::native_shell::runtime_contract::{BrowserPillModel, BrowserPillState};

/// Return the sidebar pill-editor input hit rectangle when it is visible.
pub(in crate::app_core::native_shell::composition::state) fn sidebar_pill_editor_input_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
) -> Option<Rect> {
    let rect = sidebar_workspace_sections(layout, style).tags;
    (rect.width() > 1.0 && rect.height() > 1.0).then(|| sidebar_tag_input_rect(rect, style.sizing))
}

/// Return the sidebar pill-editor text hit rectangle when it is visible.
pub(in crate::app_core::native_shell::composition::state) fn sidebar_pill_editor_text_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
) -> Option<Rect> {
    sidebar_pill_editor_input_rect(layout, style)
        .map(|rect| inset_rect(rect, style.sizing.text_inset_x, style.sizing.text_inset_y))
}

/// Resolve a sidebar tag-editor point to its UI action.
pub(super) fn sidebar_tag_action_at_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let rect = sidebar_workspace_sections(layout, style).tags;
    if rect.width() <= 1.0 || rect.height() <= 1.0 || !rect.contains(point) {
        return None;
    }
    if sidebar_tag_input_rect(rect, style.sizing).contains(point) {
        return Some(UiAction::FocusBrowserPillEditorInput);
    }
    for (pill, pill_rect) in sidebar_tag_pill_rects(rect, style.sizing, model) {
        if pill_rect.contains(point) {
            return Some(UiAction::ToggleBrowserPillOption {
                label: pill.id.clone(),
            });
        }
    }
    None
}

/// Return the local sidebar tag input rectangle.
fn sidebar_tag_input_rect(rect: Rect, sizing: SizingTokens) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let height = sizing.browser_row_height.max(18.0);
    Rect::from_min_max(
        Point::new(
            rect.min.x + pad,
            (rect.max.y - pad - height).max(rect.min.y + pad),
        ),
        Point::new(rect.max.x - pad, rect.max.y - pad),
    )
}

/// Return visible sidebar tag pill rectangles paired with their pill models.
fn sidebar_tag_pill_rects(
    rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Vec<(&BrowserPillModel, Rect)> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 3.0;
    let title_height = sizing.font_meta + sizing.text_inset_y + 2.0;
    let input = sidebar_tag_input_rect(rect, sizing);
    let row_height = sizing.browser_row_height.max(18.0);
    let col_width = ((rect.width() - pad * 2.0 - gap) * 0.5).max(36.0);
    let mut pills: Vec<_> = model.browser.pill_editor().accepted_pills.iter().collect();
    if pills.is_empty() {
        pills.extend(
            model
                .browser
                .pill_editor()
                .option_pills
                .iter()
                .filter(|pill| !matches!(pill.state, BrowserPillState::Off))
                .take(4),
        );
    }
    if pills.is_empty() {
        pills.extend(model.browser.pill_editor().option_pills.iter().take(4));
    }
    if let Some(create) = model.browser.pill_editor().create_pill.as_ref() {
        pills.push(create);
    }
    pills
        .into_iter()
        .take(12)
        .enumerate()
        .filter_map(|(index, pill)| {
            let col = index % 2;
            let row = index / 2;
            let min_x = rect.min.x + pad + (col_width + gap) * col as f32;
            let min_y = rect.min.y + pad + title_height + (row_height + gap) * row as f32;
            let pill_rect = Rect::from_min_max(
                Point::new(min_x, min_y),
                Point::new(
                    (min_x + col_width).min(rect.max.x - pad),
                    min_y + row_height,
                ),
            );
            (pill_rect.max.y <= input.min.y - gap).then_some((pill, pill_rect))
        })
        .collect()
}

/// Inset a rectangle without inverting its bounds.
fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(
            (rect.min.x + x).min(rect.max.x),
            (rect.min.y + y).min(rect.max.y),
        ),
        Point::new(
            (rect.max.x - x).max(rect.min.x),
            (rect.max.y - y).max(rect.min.y),
        ),
    )
}
