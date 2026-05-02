use super::*;

#[test]
fn layout_exposes_non_overlapping_columns() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert!(layout.columns[0].max.x <= layout.columns[1].min.x);
    assert!(layout.columns[1].max.x <= layout.columns[2].min.x);
    assert!(layout.columns.iter().all(|column| column.width() > 40.0));
}

#[test]
fn hit_test_prefers_column_node_inside_content() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let center = Point::new(
        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
        (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
    );
    assert_eq!(
        layout.hit_test(center),
        Some(layout::ShellNodeKind::BrowserTable)
    );
}

#[test]
fn primary_click_selects_clicked_column() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let point = Point::new(
        (layout.columns[2].min.x + layout.columns[2].max.x) * 0.5,
        (layout.columns[2].min.y + layout.columns[2].max.y) * 0.5,
    );
    assert!(state.handle_primary_click(&layout, point));
    let frame = state.build_frame(&layout, &crate::compat_app_contract::AppModel::default());
    assert!(frame.primitives.len() > 10);
    assert!(!frame.text_runs.is_empty());
}

#[test]
fn arrow_keys_wrap_selection() {
    let mut state = NativeShellState::new();
    assert!(state.handle_key(KeyCode::ArrowRight));
    assert!(state.handle_key(KeyCode::ArrowRight));
    assert!(state.handle_key(KeyCode::ArrowRight));
    assert!(state.handle_key(KeyCode::ArrowLeft));
}

#[test]
fn browser_row_hit_test_resolves_visible_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model
        .browser
        .rows
        .push(crate::compat_app_contract::BrowserRowModel::new(
            7, "kick", 0, false, true,
        ));
    let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
    let row_center_y = layout.browser_rows.min.y + (style.sizing.browser_row_height * 0.5);
    let point = Point::new(
        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
        row_center_y,
    );
    assert_eq!(state.browser_row_at_point(&layout, &model, point), Some(7));
    state.sync_from_model(&model);
    let frame = state.build_frame(&layout, &model);
    assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
}

#[test]
fn compact_layout_keeps_tight_header_and_footer_bands() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert!(layout.top_bar.height() <= 40.0);
    assert!(layout.status_bar.height() <= 24.0);
    assert!(layout.waveform_card.height() >= 120.0);
}

#[test]
fn compact_layout_preserves_content_on_narrow_viewports() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    assert!(layout.sidebar.width() >= 160.0);
    assert!(layout.content.width() >= 200.0);
    assert!(layout.columns.iter().all(|column| column.width() >= 40.0));
}

#[test]
fn classic_reference_viewport_matches_dense_geometry_contract() {
    let viewport = Vector2::new(1440.0, 810.0);
    let style = style::StyleTokens::for_viewport_width(viewport.x);
    let layout = ShellLayout::build(viewport);
    let snapshot = layout.contract_snapshot(&style);

    assert!(snapshot.sidebar_width >= style.sizing.sidebar_min_width - 1.0);
    assert!(snapshot.sidebar_width <= style.sizing.sidebar_max_width + 1.0);
    assert!(snapshot.waveform_height >= style.sizing.waveform_min_height - 1.0);
    assert!(snapshot.waveform_height <= style.sizing.waveform_max_height + 1.0);
    assert!(snapshot.browser_row_capacity >= 22);
    assert!(snapshot.top_bar_height <= 34.0);
    assert!(snapshot.status_bar_height <= 20.0);
}

#[test]
fn layout_snapshot_clamps_to_tokenized_min_viewport() {
    let style = style::StyleTokens::for_viewport_width(0.0);
    let layout = ShellLayout::build_with_style(Vector2::new(1.0, 1.0), &style);
    let snapshot = layout.contract_snapshot(&style);
    assert_eq!(snapshot.viewport_width, style.sizing.min_viewport_width);
    assert_eq!(snapshot.viewport_height, style.sizing.min_viewport_height);
}

#[test]
fn scaled_layout_preserves_scale_ratio_for_rebuild() {
    let viewport = Vector2::new(1280.0, 720.0);
    let scaled_style = style::StyleTokens::for_viewport_with_scale(viewport.x, 1.6);
    let base_style = style::StyleTokens::for_viewport_width(viewport.x);
    let layout = ShellLayout::build_with_style(viewport, &scaled_style);

    let rebuilt_style =
        style::StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);

    let expected_scale = scaled_style.sizing.font_body / base_style.sizing.font_body;
    let observed_scale = rebuilt_style.sizing.font_body / base_style.sizing.font_body;

    assert!((layout.ui_scale - 1.6).abs() < 0.0001);
    assert!((expected_scale - observed_scale).abs() < 0.0001);
}

#[test]
fn viewport_tier_sizing_changes_row_density() {
    let narrow = style::StyleTokens::for_viewport_width(820.0);
    let wide = style::StyleTokens::for_viewport_width(2300.0);
    assert!(narrow.sizing.browser_row_height < wide.sizing.browser_row_height);
    assert!(narrow.sizing.source_row_height < wide.sizing.source_row_height);
}

#[test]
fn visual_density_snapshot_scales_across_tiers() {
    let compact_viewport = Vector2::new(820.0, 520.0);
    let standard_viewport = Vector2::new(1280.0, 720.0);
    let wide_viewport = Vector2::new(2300.0, 1080.0);

    let compact_style = style::StyleTokens::for_viewport_width(compact_viewport.x);
    let standard_style = style::StyleTokens::for_viewport_width(standard_viewport.x);
    let wide_style = style::StyleTokens::for_viewport_width(wide_viewport.x);

    let compact = ShellLayout::build(compact_viewport).contract_snapshot(&compact_style);
    let standard = ShellLayout::build(standard_viewport).contract_snapshot(&standard_style);
    let wide = ShellLayout::build(wide_viewport).contract_snapshot(&wide_style);

    assert!(compact.top_bar_height >= standard.top_bar_height);
    assert!(wide.top_bar_height >= standard.top_bar_height);
    assert!(compact.status_bar_height >= standard.status_bar_height);
    assert!(wide.status_bar_height >= standard.status_bar_height);
    assert!(compact.browser_row_capacity <= standard.browser_row_capacity);
    assert!(standard.browser_row_capacity <= wide.browser_row_capacity);
}

#[test]
fn layout_bands_stay_within_panel_bounds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert_eq!(layout.top_bar_title_row, layout.top_bar);
    assert_eq!(layout.top_bar_controls_row, layout.top_bar);
    assert!(layout.sidebar_header.max.y <= layout.sidebar_rows.min.y);
    assert!(layout.sidebar_rows.max.y <= layout.sidebar_footer.min.y);
    assert_eq!(layout.waveform_header.max.y, layout.waveform_plot.min.y);
    assert_eq!(
        layout.waveform_plot.max.y,
        layout.waveform_scrollbar_lane.min.y
    );
    assert_eq!(
        layout.waveform_scrollbar_lane.max.y,
        layout.waveform_card.max.y
    );
    assert!(layout.browser_tabs.max.y <= layout.browser_toolbar.min.y);
    assert!(layout.browser_toolbar.max.y <= layout.browser_table_header.min.y);
    assert!(layout.browser_table_header.max.y <= layout.browser_rows.min.y);
    assert!(layout.browser_rows.max.y <= layout.browser_footer.min.y);
    for index in 0..3 {
        assert!(layout.column_headers[index].max.y <= layout.column_rows[index].min.y);
        assert!(layout.column_rows[index].min.x >= layout.columns[index].min.x);
        assert!(layout.column_rows[index].max.x <= layout.columns[index].max.x);
    }
}

#[test]
fn waveform_view_uses_side_insets_inside_card_body() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert_eq!(
        layout.waveform_plot.min.x - layout.waveform_card.min.x,
        10.0
    );
    assert_eq!(
        layout.waveform_card.max.x - layout.waveform_plot.max.x,
        10.0
    );
    assert_eq!(
        layout.waveform_scrollbar_lane.min.x - layout.waveform_card.min.x,
        10.0
    );
    assert_eq!(
        layout.waveform_card.max.x - layout.waveform_scrollbar_lane.max.x,
        10.0
    );
}

#[test]
fn major_panels_share_edges_without_gap() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert_eq!(layout.top_bar.max.y, layout.sidebar.min.y);
    assert_eq!(layout.top_bar.max.y, layout.content.min.y);
    assert_eq!(layout.sidebar.max.x, layout.content.min.x);
    assert_eq!(layout.waveform_card.max.y, layout.browser_panel.min.y);
    assert_eq!(layout.sidebar.max.y, layout.status_bar.min.y);
    assert_eq!(layout.browser_panel.max.y, layout.status_bar.min.y);
}

#[test]
fn browser_bands_fill_browser_panel_width_without_inner_gutters() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert_eq!(layout.browser_tabs.min.x, layout.browser_panel.min.x);
    assert_eq!(layout.browser_tabs.max.x, layout.browser_panel.max.x);
    assert_eq!(layout.browser_toolbar.min.x, layout.browser_panel.min.x);
    assert_eq!(layout.browser_toolbar.max.x, layout.browser_panel.max.x);
    assert_eq!(
        layout.browser_table_header.min.x,
        layout.browser_panel.min.x
    );
    assert_eq!(
        layout.browser_table_header.max.x,
        layout.browser_panel.max.x
    );
    assert_eq!(layout.browser_rows.min.x, layout.browser_panel.min.x);
    assert_eq!(layout.browser_rows.max.x, layout.browser_panel.max.x);
    assert_eq!(layout.browser_footer.min.x, layout.browser_panel.min.x);
    assert_eq!(layout.browser_footer.max.x, layout.browser_panel.max.x);
}

#[test]
fn top_bar_clusters_stay_ordered_and_inside_bar() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert!(layout.top_bar_title_cluster.min.x >= layout.top_bar.min.x);
    assert!(layout.top_bar_title_cluster.max.y <= layout.top_bar_title_row.max.y);
    assert!(layout.top_bar_action_cluster.min.x >= layout.top_bar.min.x);
    assert!(layout.top_bar_action_cluster.max.y <= layout.top_bar_title_row.max.y);
    assert!(layout.top_bar_title_cluster.max.x <= layout.top_bar_action_cluster.min.x);
}

#[test]
fn top_bar_clusters_reserve_minimum_title_and_action_widths() {
    let viewport = Vector2::new(1280.0, 720.0);
    let tokens = style::StyleTokens::for_viewport_width(viewport.x);
    let layout = ShellLayout::build(viewport);
    assert!(
        layout.top_bar_action_cluster.width()
            >= tokens.sizing.top_bar_action_cluster_min_width - 1.0
    );
    assert!(
        layout.top_bar_title_cluster.width()
            >= tokens.sizing.top_bar_action_cluster_title_reserve_width - 1.0
    );
}

#[test]
fn status_segments_remain_non_overlapping_and_bounded() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    assert!(layout.status_left_segment.min.x >= layout.status_bar.min.x);
    assert!(layout.status_progress_segment.max.x <= layout.status_bar.max.x);
    assert!(layout.status_left_segment.max.x <= layout.status_center_segment.min.x);
    assert!(layout.status_center_segment.max.x <= layout.status_right_segment.min.x);
    assert!(layout.status_right_segment.max.x <= layout.status_progress_segment.min.x);
    assert!(layout.status_left_segment.max.y <= layout.status_bar.max.y);
    assert!(layout.status_center_segment.max.y <= layout.status_bar.max.y);
    assert!(layout.status_right_segment.max.y <= layout.status_bar.max.y);
    assert!(layout.status_progress_segment.max.y <= layout.status_bar.max.y);
}

#[test]
fn layout_uses_tokenized_shell_heights() {
    let width = 1280.0;
    let height = 720.0;
    let layout = ShellLayout::build(Vector2::new(width, height));
    let tokens = style::StyleTokens::for_viewport_width(width);
    assert!((layout.top_bar.height() - tokens.sizing.top_bar_height).abs() < 0.001);
    assert!((layout.status_bar.height() - tokens.sizing.status_bar_height).abs() < 0.001);
}

#[test]
fn browser_header_band_can_fit_single_metadata_line_across_tiers() {
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let tokens = style::StyleTokens::for_viewport_width(viewport.x);
        let layout = ShellLayout::build(viewport);
        let centered_y = layout.browser_table_header.min.y
            + ((layout.browser_table_header.height() - tokens.sizing.font_meta).max(0.0) * 0.5);
        let top = centered_y.max(layout.browser_table_header.min.y + tokens.sizing.text_inset_y);
        assert!(top + tokens.sizing.font_meta <= layout.browser_table_header.max.y + 0.5);
    }
}

#[test]
fn wide_viewport_renders_more_browser_rows_than_narrow_viewport() {
    let narrow_layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let wide_layout = ShellLayout::build(Vector2::new(2300.0, 1080.0));
    let mut state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    for index in 0..40 {
        model
            .browser
            .rows
            .push(crate::compat_app_contract::BrowserRowModel::new(
                index,
                format!("row_{index:02}"),
                1,
                false,
                false,
            ));
    }
    state.sync_from_model(&model);
    let narrow_frame = state.build_frame(&narrow_layout, &model);
    let wide_frame = state.build_frame(&wide_layout, &model);
    let narrow_rows = narrow_frame
        .text_runs
        .iter()
        .filter(|run| run.text.starts_with("row_"))
        .count();
    let wide_rows = wide_frame
        .text_runs
        .iter()
        .filter(|run| run.text.starts_with("row_"))
        .count();
    assert!(wide_rows > narrow_rows);
}
