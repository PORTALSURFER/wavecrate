use super::filters::sidebar_filter_row_rects;
use super::*;

#[path = "dropdown/options.rs"]
mod options;

use self::options::sidebar_filter_dropdown_buttons;

/// Build sidebar filter dropdown panel geometry and option buttons.
pub(in crate::app_core::native_shell::composition::state) fn sidebar_filter_dropdown_spec(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    dropdown: Option<SidebarFilterDropdownState>,
) -> Option<(Rect, Vec<ActionButton>)> {
    let dropdown = dropdown?;
    let filters_rect = sidebar_workspace_sections(layout, style).filters;
    let rows = sidebar_filter_row_rects(filters_rect, style.sizing);
    let row = rows
        .get(sidebar_filter_dropdown_row_index(dropdown.facet))
        .copied()?;
    let definitions = sidebar_filter_dropdown_buttons(dropdown.facet, model, style);
    if definitions.is_empty() {
        return None;
    }
    let sizing = style.sizing;
    let panel_padding = sizing.panel_inset.max(4.0);
    let button_width = row.width().max(132.0);
    let button_height = sizing.sidebar_action_button_height.max(18.0);
    let button_gap = sizing.sidebar_action_button_gap.max(2.0);
    let panel_width = button_width + panel_padding * 2.0;
    let panel_height = (button_height * definitions.len() as f32)
        + (button_gap * definitions.len().saturating_sub(1) as f32)
        + panel_padding * 2.0;
    let min_x = layout.sidebar.min.x + sizing.panel_inset;
    let max_x = (layout.sidebar.max.x - sizing.panel_inset - panel_width).max(min_x);
    let below_y = row.max.y + sizing.border_width.max(1.0);
    let above_y = row.min.y - panel_height - sizing.border_width.max(1.0);
    let min_y = layout.sidebar.min.y + sizing.panel_inset;
    let max_y = (layout.sidebar.max.y - sizing.panel_inset - panel_height).max(min_y);
    let preferred_y = if below_y <= max_y { below_y } else { above_y };
    let panel_min = Point::new(
        row.min.x.clamp(min_x, max_x),
        preferred_y.clamp(min_y, max_y),
    );
    let panel_rect = Rect::from_min_max(
        panel_min,
        Point::new(panel_min.x + panel_width, panel_min.y + panel_height),
    );
    Some((
        panel_rect,
        layout_dropdown_buttons(panel_rect, definitions, style),
    ))
}

/// Return the rendered filter-row index for a dropdown facet.
fn sidebar_filter_dropdown_row_index(facet: SidebarFilterDropdownFacet) -> usize {
    match facet {
        SidebarFilterDropdownFacet::Format => 0,
        SidebarFilterDropdownFacet::BitDepth => 1,
        SidebarFilterDropdownFacet::Channels => 2,
        SidebarFilterDropdownFacet::Bpm => 3,
        SidebarFilterDropdownFacet::Key => 4,
        SidebarFilterDropdownFacet::Rating => 5,
    }
}

/// Apply panel-local button rectangles to one dropdown's option buttons.
fn layout_dropdown_buttons(
    panel_rect: Rect,
    definitions: Vec<ActionButton>,
    style: &StyleTokens,
) -> Vec<ActionButton> {
    let sizing = style.sizing;
    let panel_padding = sizing.panel_inset.max(4.0);
    let button_width = panel_rect.width() - panel_padding * 2.0;
    let button_height = sizing.sidebar_action_button_height.max(18.0);
    let button_gap = sizing.sidebar_action_button_gap.max(2.0);
    let button_x = panel_rect.min.x + panel_padding;
    let mut button_y = panel_rect.min.y + panel_padding;
    let mut buttons = Vec::with_capacity(definitions.len());
    for mut button in definitions {
        button.rect = Rect::from_min_max(
            Point::new(button_x, button_y),
            Point::new(button_x + button_width, button_y + button_height),
        );
        buttons.push(button);
        button_y += button_height + button_gap;
    }
    buttons
}
