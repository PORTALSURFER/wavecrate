use super::*;

fn row_fill_color(frame: &NativeViewFrame, rect: Rect) -> Rgba8 {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(fill) if fill.rect == rect => Some(fill.color),
            _ => None,
        })
        .expect("row should emit a fill rectangle")
}

fn row_label_color(frame: &NativeViewFrame, label: &str) -> Rgba8 {
    frame
        .text_runs
        .iter()
        .find_map(|run| (run.text == label).then_some(run.color))
        .expect("row label should render")
}

fn has_fill_rect(frame: &NativeViewFrame, rect: Rect, color: Rgba8) -> bool {
    frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect: fill_rect, color: fill_color })
                if *fill_rect == rect && *fill_color == color
        )
    })
}

fn has_any_rect(frame: &NativeViewFrame, rect: Rect) -> bool {
    frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect: fill_rect, .. }) if *fill_rect == rect
        )
    })
}

fn color_luma(color: Rgba8) -> u16 {
    ((u16::from(color.r) * 54) + (u16::from(color.g) * 183) + (u16::from(color.b) * 19)) / 256
}

#[test]
fn browser_rows_use_alternating_fill_stripes_for_readability() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_even", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "row_odd", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();
    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(rendered.len() >= 2);

    let frame = state.build_frame(&layout, &model);
    let even_rect = rendered[0].rect;
    let odd_rect = rendered[1].rect;
    let even_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == even_rect => Some(rect.color),
            _ => None,
        })
        .collect();
    let odd_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == odd_rect => Some(rect.color),
            _ => None,
        })
        .collect();
    let expected_even = browser_row_stripe_fill(&style, 0);
    let expected_odd = browser_row_stripe_fill(&style, 1);
    assert!(!even_fills.is_empty(), "missing even-row fills");
    assert!(!odd_fills.is_empty(), "missing odd-row fills");
    assert!(
        even_fills.contains(&expected_even),
        "expected even-row fill {expected_even:?}, saw {even_fills:?}"
    );
    assert!(
        odd_fills.contains(&expected_odd),
        "expected odd-row fill {expected_odd:?}, saw {odd_fills:?}"
    );
    assert_ne!(expected_even, expected_odd);
}

#[test]
fn locked_browser_rows_keep_neutral_fill_and_draw_left_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "locked row", 1, false, false).with_locked(true));

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let marker_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("locked marker");
    let frame = state.build_frame(&layout, &model);
    let row_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == row.rect => Some(rect.color),
            _ => None,
        })
        .collect();

    assert!(
        row_fills.contains(&browser_row_stripe_fill(&style, 0)),
        "locked row should keep the standard stripe fill"
    );
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marker_rect && *color == style.accent_mint
        )
    }));
}

#[test]
fn marked_browser_rows_use_distinct_fill_and_draw_cyan_left_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "marked row", 1, false, false).with_marked(true));

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let marker_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("marked marker");
    let frame = state.build_frame(&layout, &model);
    let row_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == row.rect => Some(rect.color),
            _ => None,
        })
        .collect();

    assert!(
        row_fills.contains(&browser_marked_row_fill(&style, 0)),
        "marked row should render the dedicated marked fill"
    );
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marker_rect && *color == style.highlight_cyan
        )
    }));
}

#[test]
fn marked_locked_browser_rows_offset_keep_lock_marker_after_mark_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(0, "marked locked row", 1, false, false)
            .with_marked(true)
            .with_locked(true),
    );

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let marked_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("marked marker");
    let locked_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 4.0).expect("locked marker");
    let frame = state.build_frame(&layout, &model);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marked_rect && *color == style.highlight_cyan
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == locked_rect && *color == style.accent_mint
        )
    }));
}

#[test]
fn focused_browser_rows_render_similarity_button_on_far_left() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "focused row", 1, true, true));

    let button_rect = state
        .browser_similarity_button_rect(&layout, &model)
        .expect("focused row should expose a similarity button");
    let row = rendered_browser_rows(&layout, &model, &style)
        .into_iter()
        .next()
        .expect("browser row should render");
    let row_text_layout = compute_browser_row_text_layout(row.rect, style.sizing);
    let frame = state.build_frame(&layout, &model);

    assert!(
        button_rect.min.x <= row_text_layout.sample_label.min.x,
        "similarity button should stay on the far left edge of the sample column"
    );
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(FillRect { rect, .. }) if *rect == button_rect)
    }));
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| { matches!(primitive, Primitive::Image(DrawImage { .. })) })
    );
    assert!(!frame.text_runs.iter().any(|run| run.text == "SIM"));
}

#[test]
fn similarity_filtered_browser_rows_use_highlighted_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.similarity_filtered = true;
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "anchor", 1, true, true));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "match", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let frame = state.build_frame(&layout, &model);
    let anchor_rect = rendered[0].rect;
    let match_rect = rendered[1].rect;
    let anchor_fill = browser_similarity_row_fill(&style, 0, true);
    let match_fill = browser_similarity_row_fill(&style, 1, false);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == anchor_rect && *color == anchor_fill
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == match_rect && *color == match_fill
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == rendered[0].text_layout.columns.index
                    && *color == similarity_anchor_browser_index_fill(&style)
        )
    }));
}

#[test]
fn similarity_filtered_browser_rows_render_compact_similarity_strength_bars() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.similarity_filtered = true;
    model.browser.rows.push(
        BrowserRowModel::new(0, "anchor", 1, true, true).with_similarity_display_strength(1.0),
    );
    model.browser.rows.push(
        BrowserRowModel::new(1, "close", 1, false, false)
            .with_bucket_label("165 BPM")
            .with_similarity_display_strength(0.35),
    );
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let frame = state.build_frame(&layout, &model);
    let track_color = translucent_overlay_color(style.surface_overlay, style.text_muted, 0.12);
    let fill_color = blend_color(style.highlight_cyan_soft, style.highlight_cyan, 0.38);
    let anchor_track =
        browser_similarity_strength_track_rect(rendered[0].text_layout.sample_label, style.sizing)
            .expect("anchor track");
    let anchor_fill = browser_similarity_strength_fill_rect(
        anchor_track,
        rendered[0].similarity_display_strength.unwrap(),
    )
    .expect("anchor fill");
    let close_track =
        browser_similarity_strength_track_rect(rendered[1].text_layout.sample_label, style.sizing)
            .expect("close track");
    let close_fill = browser_similarity_strength_fill_rect(
        close_track,
        rendered[1].similarity_display_strength.unwrap(),
    )
    .expect("close fill");

    assert!(has_fill_rect(&frame, anchor_track, track_color));
    assert!(has_fill_rect(&frame, anchor_fill, fill_color));
    assert!(has_fill_rect(&frame, close_track, track_color));
    assert!(has_fill_rect(&frame, close_fill, fill_color));
    assert!(anchor_fill.width() > close_fill.width());
    assert!(anchor_track.width() >= 36.0);
    assert!(anchor_track.height() >= 6.0);
    assert!(
        rendered[1]
            .inline_tag_rects
            .iter()
            .all(|rect| rect.max.x <= close_track.min.x)
    );
}

#[test]
fn browser_rows_skip_similarity_strength_bar_without_strength_value() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.similarity_filtered = true;
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "plain row", 1, false, false));

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let frame = state.build_frame(&layout, &model);
    let hypothetical_track =
        browser_similarity_strength_track_rect(rendered[0].text_layout.sample_label, style.sizing)
            .expect("hypothetical track");

    assert!(!has_any_rect(&frame, hypothetical_track));
}

#[test]
fn browser_playback_age_buckets_render_distinct_left_markers_and_keep_rows_neutral() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "Fresh row", 1, false, false));
    model.browser.rows.push(
        BrowserRowModel::new(1, "Week row", 1, false, false)
            .with_playback_age_bucket(crate::compat_app_contract::PlaybackAgeBucket::OlderThanWeek),
    );
    model.browser.rows.push(
        BrowserRowModel::new(2, "Month row", 1, false, false).with_playback_age_bucket(
            crate::compat_app_contract::PlaybackAgeBucket::OlderThanMonth,
        ),
    );
    model.browser.rows.push(
        BrowserRowModel::new(3, "Never row", 1, false, false)
            .with_playback_age_bucket(crate::compat_app_contract::PlaybackAgeBucket::NeverPlayed),
    );
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let frame = state.build_frame(&layout, &model);
    let fresh_fill = row_fill_color(&frame, rendered[0].rect);
    let week_fill = row_fill_color(&frame, rendered[1].rect);
    let month_fill = row_fill_color(&frame, rendered[2].rect);
    let never_fill = row_fill_color(&frame, rendered[3].rect);
    let fresh_text = row_label_color(&frame, "Fresh row");
    let week_text = row_label_color(&frame, "Week row");
    let month_text = row_label_color(&frame, "Month row");
    let never_text = row_label_color(&frame, "Never row");
    let fresh_marker_rect = browser_playback_age_marker_rect(rendered[0].rect, style.sizing, 0.0)
        .expect("fresh marker");
    let week_marker_rect =
        browser_playback_age_marker_rect(rendered[1].rect, style.sizing, 0.0).expect("week marker");
    let month_marker_rect = browser_playback_age_marker_rect(rendered[2].rect, style.sizing, 0.0)
        .expect("month marker");
    let never_marker_rect = browser_playback_age_marker_rect(rendered[3].rect, style.sizing, 0.0)
        .expect("never marker");
    let fresh_marker_color = browser_playback_age_marker_color(
        &style,
        crate::compat_app_contract::PlaybackAgeBucket::Fresh,
    );
    let week_marker_color = browser_playback_age_marker_color(
        &style,
        crate::compat_app_contract::PlaybackAgeBucket::OlderThanWeek,
    );
    let month_marker_color = browser_playback_age_marker_color(
        &style,
        crate::compat_app_contract::PlaybackAgeBucket::OlderThanMonth,
    );
    let never_marker_color = browser_playback_age_marker_color(
        &style,
        crate::compat_app_contract::PlaybackAgeBucket::NeverPlayed,
    );

    assert_eq!(fresh_fill, browser_row_stripe_fill(&style, 0));
    assert_eq!(week_fill, browser_row_stripe_fill(&style, 1));
    assert_eq!(month_fill, browser_row_stripe_fill(&style, 2));
    assert_eq!(never_fill, browser_row_stripe_fill(&style, 3));
    assert_eq!(fresh_text, style.text_primary);
    assert_eq!(week_text, style.text_primary);
    assert_eq!(month_text, style.text_primary);
    assert_eq!(never_text, style.text_primary);

    assert!(has_fill_rect(&frame, fresh_marker_rect, fresh_marker_color));
    assert!(has_fill_rect(&frame, week_marker_rect, week_marker_color));
    assert!(has_fill_rect(&frame, month_marker_rect, month_marker_color));
    assert!(has_fill_rect(&frame, never_marker_rect, never_marker_color));

    assert_ne!(fresh_marker_color, week_marker_color);
    assert_ne!(week_marker_color, month_marker_color);
    assert_ne!(month_marker_color, never_marker_color);
    assert!(color_luma(fresh_marker_color) > color_luma(week_marker_color));
    assert!(color_luma(week_marker_color) > color_luma(month_marker_color));
    assert!(color_luma(month_marker_color) > color_luma(never_marker_color));
}

#[test]
fn selected_browser_rows_keep_playback_age_marker_in_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(0, "selected month row", 1, true, false).with_playback_age_bucket(
            crate::compat_app_contract::PlaybackAgeBucket::OlderThanMonth,
        ),
    );

    let row_rect = rendered_browser_rows(&layout, &model, &style)[0].rect;
    state.sync_from_model(&model);
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    let selected_fill = selected_browser_row_fill(&style);
    let marker_rect =
        browser_playback_age_marker_rect(row_rect, style.sizing, 0.0).expect("marker rect");
    let marker_color = browser_playback_age_marker_color(
        &style,
        crate::compat_app_contract::PlaybackAgeBucket::OlderThanMonth,
    );

    assert!(has_fill_rect(&overlay, row_rect, selected_fill));
    assert!(has_fill_rect(&overlay, marker_rect, marker_color));
}
