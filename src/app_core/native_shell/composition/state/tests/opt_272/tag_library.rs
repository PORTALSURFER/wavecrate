use super::*;
use crate::app_core::native_shell::runtime_contract::{
    BrowserPillModel, BrowserPillState, UiAction,
};
use crate::gui::types::Point;

#[test]
fn compact_tag_section_expand_button_opens_tag_library() {
    let mut state = NativeShellState::new();
    let model = populated_single_sidebar_model();
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let tags = sidebar_workspace_sections(&layout, &style).tags;
    let point = Point::new(tags.max.x - 12.0, tags.min.y + 10.0);

    assert_eq!(
        state.source_action_at_point(&layout, &model, point),
        Some(UiAction::ToggleBrowserPillEditor)
    );
}

#[test]
fn expanded_tag_library_routes_used_tag_rows_before_browser_rows() {
    let mut state = NativeShellState::new();
    let mut model = browser_model_with_rows(32, 0);
    model.browser_actions.pill_editor_open = true;
    model
        .browser
        .pill_editor
        .option_pills
        .push(BrowserPillModel {
            id: String::from("Bass"),
            label: String::from("Bass"),
            state: BrowserPillState::Off,
        });
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let point = first_tag_library_option_point(&layout, &style);

    assert_eq!(
        state.tag_library_action_at_point(&layout, &model, point),
        Some(UiAction::ToggleBrowserPillOption {
            label: String::from("Bass")
        })
    );
}

fn first_tag_library_option_point(layout: &ShellLayout, style: &StyleTokens) -> Point {
    let sizing = style.sizing;
    let panel_gap = sizing.panel_gap.max(3.0);
    let panel_width = layout
        .content
        .width()
        .mul_add(0.28, 0.0)
        .clamp(190.0, 270.0)
        .min((layout.content.width() - panel_gap).max(0.0));
    let panel_min = Point::new(layout.sidebar.max.x + panel_gap, layout.sidebar.min.y);
    let row_height = sizing.browser_row_height.max(18.0);
    let row_gap = sizing.border_width.max(1.0) + 1.0;
    let header_height = row_height + 5.0;
    let group_title_height = sizing.font_meta + 2.0;
    let playback_title_y = panel_min.y + header_height + sizing.panel_inset.max(5.0);
    let playback_rows_top = playback_title_y + group_title_height + row_gap;
    let tags_title_y = playback_rows_top + row_height * 2.0 + row_gap * 2.0 + 3.0;
    let first_row_top = tags_title_y + group_title_height + row_gap;
    Point::new(
        panel_min.x + panel_width.min(32.0),
        first_row_top + row_height * 0.5,
    )
}
