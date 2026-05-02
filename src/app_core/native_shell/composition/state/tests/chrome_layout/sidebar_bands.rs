use super::super::*;
use crate::compat_app_contract::FolderPaneIdModel;

#[test]
fn sidebar_sections_keep_equal_height_panes_across_viewports() {
    let sizes = [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ];
    let mut state = NativeShellState::new();
    let model = populated_sidebar_model();
    for viewport in sizes {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let sections = sidebar_sections(&layout, &style, &model);
        let rendered_upper_sources =
            state.rendered_source_row_rects_for_pane(&layout, &model, FolderPaneIdModel::Upper);
        let rendered_lower_sources =
            state.rendered_source_row_rects_for_pane(&layout, &model, FolderPaneIdModel::Lower);
        assert_rect_inside(layout.sidebar_rows, sections.upper.bounds);
        assert_rect_inside(layout.sidebar_rows, sections.lower.bounds);
        assert!((sections.upper.bounds.height() - sections.lower.bounds.height()).abs() <= 0.01);
        assert!((sections.upper.bounds.max.y - sections.lower.bounds.min.y).abs() <= 0.01);
        assert!(!rendered_upper_sources.is_empty());
        assert!(!rendered_lower_sources.is_empty());
    }
}

#[test]
fn sidebar_sections_keep_each_pane_contents_inside_its_half_when_cramped() {
    let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
    let style = style_for_layout(&layout);
    let model = populated_sidebar_model();
    let sections = sidebar_sections(&layout, &style, &model);
    for pane_sections in [sections.upper, sections.lower] {
        assert_rect_inside(layout.sidebar_rows, pane_sections.bounds);
        assert_rect_inside(pane_sections.bounds, pane_sections.source_rows);
        assert_rect_inside(pane_sections.bounds, pane_sections.folder_header);
        assert_rect_inside(pane_sections.bounds, pane_sections.tree_rows);
        assert!(pane_sections.source_rows.max.y <= pane_sections.folder_header.min.y);
        assert!(pane_sections.folder_header.max.y <= pane_sections.tree_rows.min.y);
    }
}

#[test]
fn sidebar_header_and_footer_surfaces_stay_ordered_across_density_tiers() {
    let model = populated_sidebar_model();
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let header = resolve_sidebar_header_surface_layout(
            layout.sidebar_header,
            style.sizing,
            &sidebar_header_surface_content(&model),
        );
        let footer = resolve_sidebar_footer_surface_layout(
            layout.sidebar_footer,
            style.sizing,
            &sidebar_footer_surface_content(&model, 4, 5),
        );
        assert_rect_inside(layout.sidebar_header, header.title_text_rect);
        assert_rect_inside(layout.sidebar_header, header.query_text_rect);
        let add = header.add_button_rect.expect("sidebar add button");
        assert_rect_inside(layout.sidebar_header, add);
        assert!(header.title_text_rect.max.x <= add.min.x);
        assert!(header.query_text_rect.max.y <= layout.sidebar_header.max.y);

        assert_rect_inside(layout.sidebar_footer, footer.primary_text_rect);
        assert_rect_inside(layout.sidebar_footer, footer.secondary_text_rect);
        for button in &footer.action_buttons {
            assert_rect_inside(layout.sidebar_footer, button.rect);
        }
        for pair in footer.action_buttons.windows(2) {
            assert!(pair[0].rect.max.x <= pair[1].rect.min.x);
        }
    }
}
