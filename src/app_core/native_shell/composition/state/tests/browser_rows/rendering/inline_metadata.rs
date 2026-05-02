use super::*;

#[test]
fn browser_inline_metadata_prefers_explicit_row_metadata() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "Kick 01", 1, true, true).with_bucket_label("165 BPM"));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("165 BPM"))
    );
}

#[test]
fn browser_inline_metadata_tags_render_chip_backgrounds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(0, "Kick 01", 1, true, true)
            .with_bucket_label("165 BPM · LOOP · LONG"),
    );
    let frame = state.build_frame(&layout, &model);
    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let row_text_layout = compute_browser_row_text_layout(row.rect, style.sizing);
    let expected_chip_rects = browser_inline_tag_chip_rects(
        row_text_layout.sample_label,
        &row.bucket_label,
        0.0,
        style.sizing,
    );
    assert_eq!(expected_chip_rects.len(), 3);
    assert_eq!(row.inline_tag_labels, vec!["165 BPM", "LOOP", "LONG"]);
    assert_eq!(row.inline_tag_rects, expected_chip_rects);
    for rect in expected_chip_rects {
        assert!(frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect: primitive_rect, color })
                    if *primitive_rect == rect
                        && *color == blend_color(style.surface_overlay, style.bg_tertiary, 0.54)
            )
        }));
    }
    for label in ["165 BPM", "LOOP", "LONG"] {
        assert!(frame.text_runs.iter().any(|run| run.text == label));
    }
    assert_eq!(row.visible_row_label, "0");
}

#[test]
fn browser_inline_metadata_chip_rects_fit_short_sample_labels() {
    let sample_label = Rect::from_min_max(Point::new(24.0, 10.0), Point::new(164.0, 19.0));
    let rects = browser_inline_tag_chip_rects(
        sample_label,
        "165 BPM · LOOP",
        0.0,
        style_for_layout(&ShellLayout::build(Vector2::new(1280.0, 720.0))).sizing,
    );

    assert_eq!(rects.len(), 2);
    for rect in rects {
        assert!(rect.min.x.is_finite());
        assert!(rect.min.y.is_finite());
        assert!(rect.max.x.is_finite());
        assert!(rect.max.y.is_finite());
        assert!(rect.width() >= 0.0);
        assert!(rect.height() >= 0.0);
        assert_rect_inside(sample_label, rect);
    }
}

#[test]
fn browser_inline_metadata_chip_rects_follow_cramped_row_text_layout() {
    let style = style_for_layout(&ShellLayout::build(Vector2::new(1280.0, 720.0)));
    let row_rect = Rect::from_min_max(Point::new(20.0, 40.0), Point::new(260.0, 50.0));
    let row_text_layout = compute_browser_row_text_layout(row_rect, style.sizing);
    let rects = browser_inline_tag_chip_rects(
        row_text_layout.sample_label,
        "165 BPM · LOOP",
        0.0,
        style.sizing,
    );

    assert_eq!(rects.len(), 2);
    for rect in rects {
        assert_rect_inside(row_text_layout.sample_label, rect);
        assert!(rect.height() <= row_text_layout.sample_label.height());
    }
}
