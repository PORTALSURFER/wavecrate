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
        return tag_library_action_at_point(layout, style, model, point);
    }
    if sidebar_tag_expand_button_rect(rect, style.sizing).contains(point) {
        return Some(UiAction::ToggleBrowserPillEditor);
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

/// Resolve a point in the expanded tag-library panel to its UI action.
pub(super) fn tag_library_action_at_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let panel = tag_library_panel_rect(layout, style.sizing, model)?;
    if !panel.contains(point) {
        return None;
    }
    if tag_library_close_rect(tag_library_header_rect(panel, style.sizing), style.sizing)
        .contains(point)
    {
        return Some(UiAction::ToggleBrowserPillEditor);
    }
    for (index, _pill) in model
        .browser
        .pill_editor()
        .exclusive_pills
        .iter()
        .enumerate()
    {
        if tag_library_playback_row_rect(panel, style.sizing, index).contains(point) {
            return Some(UiAction::SetBrowserSidebarLooped { looped: index == 0 });
        }
    }
    for (pill, row) in tag_library_option_row_rects(panel, style.sizing, model) {
        if row.contains(point) {
            return Some(UiAction::ToggleBrowserPillOption {
                label: pill.id.clone(),
            });
        }
    }
    if let Some(create) = model.browser.pill_editor().create_pill.as_ref() {
        if tag_library_create_row_rect(panel, style.sizing, model).contains(point) {
            return Some(UiAction::ToggleBrowserPillOption {
                label: create.id.clone(),
            });
        }
    }
    if sidebar_tag_input_rect(panel, style.sizing).contains(point) {
        return Some(UiAction::FocusBrowserPillEditorInput);
    }
    Some(UiAction::FocusBrowserPillEditorInput)
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

/// Return the compact tag-section expand button rectangle.
fn sidebar_tag_expand_button_rect(rect: Rect, sizing: SizingTokens) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let side = (sizing.font_meta + 6.0).max(14.0);
    Rect::from_min_max(
        Point::new(rect.max.x - pad - side, rect.min.y + sizing.text_inset_y),
        Point::new(rect.max.x - pad, rect.min.y + sizing.text_inset_y + side),
    )
}

/// Return the expanded tag-library panel rectangle.
fn tag_library_panel_rect(
    layout: &ShellLayout,
    sizing: SizingTokens,
    model: &AppModel,
) -> Option<Rect> {
    if !model.browser_actions.pill_editor_open() {
        return None;
    }
    let gap = sizing.panel_gap.max(3.0);
    let width = layout
        .content
        .width()
        .mul_add(0.28, 0.0)
        .clamp(190.0, 270.0)
        .min((layout.content.width() - gap).max(0.0));
    (width > 1.0).then(|| {
        Rect::from_min_max(
            Point::new(layout.sidebar.max.x + gap, layout.sidebar.min.y),
            Point::new(layout.sidebar.max.x + gap + width, layout.sidebar.max.y),
        )
    })
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

/// Return expanded tag-library normal-tag row rectangles paired with pill models.
fn tag_library_option_row_rects(
    panel_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Vec<(&BrowserPillModel, Rect)> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 1.0;
    let row_height = sizing.browser_row_height.max(18.0);
    let mut y = tag_library_tags_title_rect(panel_rect, sizing).max.y + gap;
    let bottom = sidebar_tag_input_rect(panel_rect, sizing).min.y - gap;
    let mut rows = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for pill in model
        .browser
        .pill_editor()
        .accepted_pills
        .iter()
        .chain(model.browser.pill_editor().option_pills.iter())
    {
        if !seen.insert(pill.id.clone()) {
            continue;
        }
        let row = Rect::from_min_max(
            Point::new(panel_rect.min.x + pad, y),
            Point::new(panel_rect.max.x - pad, (y + row_height).min(bottom)),
        );
        if row.height() < row_height * 0.65 || row.max.y > bottom {
            break;
        }
        rows.push((pill, row));
        y += row_height + gap;
    }
    rows
}

fn tag_library_header_rect(panel_rect: Rect, sizing: SizingTokens) -> Rect {
    Rect::from_min_max(
        panel_rect.min,
        Point::new(
            panel_rect.max.x,
            panel_rect.min.y + (sizing.browser_row_height.max(18.0) + 5.0),
        ),
    )
}

fn tag_library_close_rect(header: Rect, sizing: SizingTokens) -> Rect {
    let side = (sizing.font_meta + 6.0).max(14.0);
    Rect::from_min_max(
        Point::new(header.max.x - side - 4.0, header.min.y + 4.0),
        Point::new(header.max.x - 4.0, header.min.y + 4.0 + side),
    )
}

fn tag_library_group_title_rect(panel_rect: Rect, sizing: SizingTokens, index: usize) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let y = tag_library_header_rect(panel_rect, sizing).max.y
        + pad
        + index as f32 * (sizing.font_meta + sizing.browser_row_height.max(18.0) * 2.0);
    Rect::from_min_max(
        Point::new(panel_rect.min.x + pad, y),
        Point::new(panel_rect.max.x - pad, y + sizing.font_meta + 2.0),
    )
}

fn tag_library_playback_row_rect(panel_rect: Rect, sizing: SizingTokens, index: usize) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 1.0;
    let row_height = sizing.browser_row_height.max(18.0);
    let y = tag_library_group_title_rect(panel_rect, sizing, 0).max.y
        + gap
        + index as f32 * (row_height + gap);
    Rect::from_min_max(
        Point::new(panel_rect.min.x + pad, y),
        Point::new(panel_rect.max.x - pad, y + row_height),
    )
}

fn tag_library_tags_title_rect(panel_rect: Rect, sizing: SizingTokens) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 4.0;
    let y = tag_library_playback_row_rect(panel_rect, sizing, 1).max.y + gap;
    Rect::from_min_max(
        Point::new(panel_rect.min.x + pad, y),
        Point::new(panel_rect.max.x - pad, y + sizing.font_meta + 2.0),
    )
}

fn tag_library_create_row_rect(panel_rect: Rect, sizing: SizingTokens, model: &AppModel) -> Rect {
    tag_library_option_row_rects(panel_rect, sizing, model)
        .last()
        .map(|(_, row)| {
            let gap = sizing.border_width.max(1.0) + 1.0;
            Rect::from_min_max(
                Point::new(row.min.x, row.max.y + gap),
                Point::new(row.max.x, row.max.y + gap + row.height()),
            )
        })
        .unwrap_or_else(|| {
            let gap = sizing.border_width.max(1.0) + 1.0;
            let title = tag_library_tags_title_rect(panel_rect, sizing);
            Rect::from_min_max(
                Point::new(title.min.x, title.max.y + gap),
                Point::new(
                    title.max.x,
                    title.max.y + gap + sizing.browser_row_height.max(18.0),
                ),
            )
        })
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
