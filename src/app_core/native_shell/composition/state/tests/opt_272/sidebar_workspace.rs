use super::*;

#[test]
/// The sidebar reserves one source list and one folder browser at all densities.
fn sidebar_sections_render_one_source_and_folder_browser_across_viewports() {
    let sizes = [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ];
    let mut state = NativeShellState::new();
    let model = populated_single_sidebar_model();
    for viewport in sizes {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let sections = sidebar_sections(&layout, &style, &model);
        let rendered_sources = state.rendered_source_row_rects(&layout, &model);
        let expected_source_rows = rendered_source_rows(&style, &model);
        assert!(sections.upper.bounds.height() > sections.lower.bounds.height());
        assert!(sections.lower.bounds.height() <= 0.01);
        assert_eq!(rendered_sources.len(), expected_source_rows);
    }
}

/// Compact sidebar workspace keeps sources, tags, and filters ordered.
#[test]
fn compact_sidebar_workspace_anchors_tags_and_filters_below_sources() {
    let model = populated_single_sidebar_model();
    for viewport in [Vector2::new(820.0, 420.0), Vector2::new(1280.0, 720.0)] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let workspace = sidebar_workspace_sections(&layout, &style);
        let sections = sidebar_sections(&layout, &style, &model);

        assert!(layout.sidebar_rows.contains(workspace.sources.center()));
        assert!(layout.sidebar_rows.contains(workspace.tags.center()));
        assert!(layout.sidebar_rows.contains(workspace.filters.center()));
        assert!(workspace.sources.max.y <= workspace.tags.min.y);
        assert!(workspace.tags.max.y <= workspace.filters.min.y);
        assert!(workspace.sources.contains(sections.upper.bounds.center()));
    }
}
