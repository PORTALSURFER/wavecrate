use super::super::*;
#[test]
/// The sidebar reserves one source list and one folder browser at all densities.
fn sidebar_sections_render_one_source_and_folder_browser_across_viewports() {
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
        let rendered_sources = state.rendered_source_row_rects(&layout, &model);
        assert_rect_inside(layout.sidebar_rows, sections.upper.bounds);
        assert_rect_inside(layout.sidebar_rows, sections.lower.bounds);
        assert!(sections.upper.bounds.height() > sections.lower.bounds.height());
        assert!(sections.lower.bounds.height() <= 0.01);
        assert!(!rendered_sources.is_empty());
    }
}

#[test]
/// Cramped sidebar layouts keep the single pane's bands inside the sidebar.
fn sidebar_sections_keep_single_pane_contents_inside_sidebar_when_cramped() {
    let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
    let style = style_for_layout(&layout);
    let model = populated_sidebar_model();
    let sections = sidebar_sections(&layout, &style, &model);
    let pane_sections = sections.upper;
    assert_rect_inside(layout.sidebar_rows, pane_sections.bounds);
    assert_rect_inside(pane_sections.bounds, pane_sections.source_rows);
    assert_rect_inside(pane_sections.bounds, pane_sections.folder_header);
    assert_rect_inside(pane_sections.bounds, pane_sections.tree_rows);
    assert!(pane_sections.source_rows.max.y <= pane_sections.folder_header.min.y);
    assert!(pane_sections.folder_header.max.y <= pane_sections.tree_rows.min.y);
}

#[test]
fn sidebar_workspace_anchors_tags_and_filters_below_sources() {
    let model = populated_sidebar_model();
    for viewport in [Vector2::new(820.0, 400.0), Vector2::new(1280.0, 720.0)] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let workspace = sidebar_workspace_sections(&layout, &style);
        let source_sections = sidebar_sections(&layout, &style, &model);

        assert_rect_inside(layout.sidebar_rows, workspace.sources);
        assert_rect_inside(layout.sidebar_rows, workspace.tags);
        assert_rect_inside(layout.sidebar_rows, workspace.filters);
        assert!(workspace.sources.max.y <= workspace.tags.min.y);
        assert!(workspace.tags.max.y <= workspace.filters.min.y);
        assert_rect_inside(workspace.sources, source_sections.upper.bounds);
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
