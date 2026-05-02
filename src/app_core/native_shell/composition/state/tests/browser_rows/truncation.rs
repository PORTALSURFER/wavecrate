use super::*;

#[test]
/// Fractional stacked-row metrics should still snap every row to stable pixel geometry.
fn browser_rows_snap_vertical_geometry_to_pixels() {
    let column = Rect::from_min_max(Point::new(10.0, 20.25), Point::new(310.0, 220.25));
    let rows = build_stacked_rows(column, 6, 1.4, 15.8);
    assert!(!rows.is_empty());
    let expected_height = rows[0].height();
    for row in rows {
        assert!(
            (row.min.y - row.min.y.round()).abs() <= 0.001,
            "row min y {} should snap to the pixel grid",
            row.min.y
        );
        assert!(
            (row.max.y - row.max.y.round()).abs() <= 0.001,
            "row max y {} should snap to the pixel grid",
            row.max.y
        );
        assert!(
            (row.height() - expected_height).abs() <= 0.001,
            "row height {} should stay stable",
            row.height()
        );
    }
}

#[test]
fn browser_rows_share_single_pixel_separator_between_adjacent_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_top", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "row_bottom", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert_eq!(rendered.len(), 2);

    let stroke = browser_row_border_stroke(&layout);
    let second_border = browser_row_border_rect(rendered[1].rect, stroke);
    let separator_count = state
        .build_frame(&layout, &model)
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.color == style.border
                    && rect.rect.min.x == second_border.min.x
                    && rect.rect.max.x == second_border.max.x
                    && rect.rect.min.y == second_border.min.y
                    && rect.rect.max.y == second_border.min.y + stroke
            }
            _ => false,
        })
        .count();

    assert_eq!(separator_count, 1);
}

#[test]
fn browser_rows_do_not_draw_extra_left_frame_edge_when_unfocused() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_plain", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let has_left_border = state
        .build_frame(&layout, &model)
        .primitives
        .iter()
        .any(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.color == style.border
                    && rect.rect.min.x == border_rect.min.x
                    && rect.rect.max.x == border_rect.min.x + stroke
                    && rect.rect.min.y == border_rect.min.y
                    && rect.rect.max.y == border_rect.max.y
            }
            _ => false,
        });

    assert!(
        !has_left_border,
        "unfocused browser rows should not add an inner left frame edge"
    );
}

#[test]
fn browser_table_header_does_not_draw_extra_left_frame_edge() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let stroke = style.sizing.border_width.max(1.0);
    let has_left_border = state
        .build_frame(&layout, &model)
        .primitives
        .iter()
        .any(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.color == style.border
                    && rect.rect.min.x == layout.browser_table_header.min.x
                    && rect.rect.max.x == layout.browser_table_header.min.x + stroke
                    && rect.rect.min.y == layout.browser_table_header.min.y
                    && rect.rect.max.y == layout.browser_table_header.max.y
            }
            _ => false,
        });

    assert!(
        !has_left_border,
        "browser table header should share the outer sidebar/content seam instead of repainting its own left edge"
    );
}

#[test]
fn missing_browser_rows_render_red_exclamation_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_missing", 1, false, false).with_missing(true));
    model.browser.visible_count = model.browser.rows.len();

    let frame = state.build_frame(&layout, &model);
    let has_marker = frame.text_runs.iter().any(|run| {
        run.text == BROWSER_MISSING_SAMPLE_MARKER
            && run.color == style.accent_trash
            && (run.font_size - style.sizing.font_body).abs() <= f32::EPSILON
    });
    assert!(has_marker, "missing row marker should be rendered in red");
}

#[test]
fn browser_row_label_truncation_uses_slotized_sample_width() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    let label =
        String::from("ultra_long_sample_label_that_should_be_truncated_by_slotized_sample_width");
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, label.clone(), 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert_eq!(rendered.len(), 1);
    let row = &rendered[0];
    let row_text_layout = compute_browser_row_text_layout(row.rect, style.sizing);
    let sample_width = row_text_layout.sample_label.width().max(20.0);
    assert_eq!(
        row.label,
        truncate_to_width(&label, sample_width, style.sizing.font_body)
    );
    assert_eq!(row.visible_row_label, "0");
    assert!(row.inline_tag_labels.is_empty());
    assert!(row.inline_tag_rects.is_empty());
}

#[test]
fn browser_row_selection_changes_reuse_cached_row_geometry() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    for index in 0..8 {
        model.browser.rows.push(
            BrowserRowModel::new(
                index,
                format!("very_long_browser_label_{index}_for_truncation_cache"),
                1,
                false,
                false,
            )
            .with_bucket_label("meta_bucket_label_that_is_also_long"),
        );
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.selected_visible_row = Some(0);
    let (first_ptr, first_selected, second_selected) = {
        let first_rows = state.cached_browser_rows(&layout, &style, &model);
        (
            first_rows.as_ptr(),
            first_rows[0].selected,
            first_rows[1].selected,
        )
    };
    let first = state.browser_row_truncation_frame_counts();
    assert!(first.lookup_count > 0);
    assert_eq!(first.cache_hit_count, 0);
    assert!(first.cache_miss_count > 0);
    assert!(first_selected);
    assert!(!second_selected);

    model.browser.rows.make_mut()[0].selected = false;
    model.browser.rows.make_mut()[1].selected = true;
    model.browser.selected_visible_row = Some(1);
    let (second_ptr, second_selected, third_selected) = {
        let second_rows = state.cached_browser_rows(&layout, &style, &model);
        (
            second_rows.as_ptr(),
            second_rows[0].selected,
            second_rows[1].selected,
        )
    };
    let second = state.browser_row_truncation_frame_counts();
    assert_eq!(second_ptr, first_ptr);
    assert!(!second_selected);
    assert!(third_selected);
    assert_eq!(second.lookup_count, 0);
    assert_eq!(second.cache_miss_count, 0);
    assert_eq!(second.cache_hit_count, 0);
}

#[test]
fn browser_row_truncation_cache_invalidates_when_row_text_revision_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(
            0,
            "very_long_browser_label_for_truncation_cache",
            1,
            false,
            false,
        )
        .with_bucket_label("bucket_label"),
    );
    model.browser.rows.push(
        BrowserRowModel::new(
            1,
            "another_very_long_browser_label_for_truncation_cache",
            1,
            false,
            false,
        )
        .with_bucket_label("bucket_label"),
    );
    model.browser.visible_count = model.browser.rows.len();
    let _ = state.cached_browser_rows(&layout, &style, &model);
    let _ = state.browser_row_truncation_frame_counts();

    model.browser.rows.make_mut()[0].label =
        String::from("updated_long_browser_label_for_cache_reset").into();
    let _ = state.cached_browser_rows(&layout, &style, &model);
    let second = state.browser_row_truncation_frame_counts();
    assert!(second.lookup_count > 0);
    assert_eq!(second.cache_hit_count, 0);
    assert!(second.cache_miss_count > 0);
}
