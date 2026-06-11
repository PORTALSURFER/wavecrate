use super::*;

fn shared_dense_row_palette() -> radiant::prelude::DenseRowPalette {
    radiant::prelude::dense_row_palette_from_style(
        &radiant::prelude::ThemeTokens::default(),
        radiant::prelude::WidgetStyle::subtle(radiant::prelude::WidgetTone::Accent),
    )
}

#[test]
fn sample_browser_rows_match_keyboard_scroll_stride() {
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let expected_names = state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    let frame = crate::native_app::test_support::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    let mut row_tops = frame
        .paint_plan
        .text_runs()
        .filter(|text| {
            expected_names
                .iter()
                .any(|name| text.text.as_str().starts_with(name))
        })
        .map(|text| text.rect.min.y)
        .collect::<Vec<_>>();
    row_tops.sort_by(|a, b| a.total_cmp(b));
    row_tops.dedup_by(|a, b| (*a - *b).abs() < 0.5);

    assert!(row_tops.len() >= 2, "{row_tops:?}");
    assert!(
        row_tops.windows(2).all(|pair| {
            ((pair[1] - pair[0]) - crate::native_app::test_support::SAMPLE_BROWSER_ROW_HEIGHT).abs()
                < 0.5
        }),
        "{row_tops:?}"
    );
}

#[test]
fn sample_browser_projection_window_matches_rendered_row_order() {
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let projection_names = {
        crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewModel::prepare_visible_sample_window(&mut state);
        let state = &state;
        let model =
            crate::native_app::app_chrome::view_models::sample_browser::SampleBrowserViewModel::from_app_state(state);
        assert_eq!(
            model.visible_samples.window.total_items,
            model.visible_samples.total_count
        );
        assert_eq!(
            model.visible_samples.rows.len(),
            model.visible_samples.window.window_len()
        );
        model
            .visible_samples
            .rows
            .iter()
            .take(4)
            .map(|row| row.file.stem.clone())
            .collect::<Vec<_>>()
    };
    let frame = crate::native_app::test_support::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    let rendered_positions = projection_names
        .iter()
        .map(|name| {
            frame
                .paint_plan
                .text_runs()
                .find(|text| text.text.as_str().starts_with(name))
                .map(|text| text.rect.min.y)
                .unwrap_or_else(|| panic!("{name} should render from projected row order"))
        })
        .collect::<Vec<_>>();

    assert!(
        rendered_positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{rendered_positions:?}"
    );
}

#[test]
fn sample_browser_keyboard_scroll_keeps_two_context_rows() {
    assert_eq!(
        crate::native_app::test_support::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
        2
    );
    assert_eq!(
        crate::native_app::test_support::SAMPLE_BROWSER_ROW_HEIGHT,
        22.0
    );
}

#[test]
fn selected_sample_browser_row_paints_strong_fill_and_left_marker() {
    let widget = sample_hit_target(true, false, false, false);
    let bounds = Rect::from_xy_size(12.0, 8.0, 240.0, 22.0);
    let plan = widget.paint_plan_with_defaults(bounds);
    let fills = plan.fill_rects().collect::<Vec<_>>();
    let selected_fill = shared_dense_row_palette()
        .selected
        .expect("dense-row selected fill");

    assert!(
        fills
            .iter()
            .any(|fill| fill.rect == bounds && fill.color == selected_fill)
    );
    assert!(fills.iter().any(|fill| {
        fill.color
            == Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 245,
            }
            && fill.rect.width() <= 3.5
    }));
}

#[test]
fn sample_browser_row_hover_paints_bright_background_without_marker() {
    let bounds = Rect::from_size(180.0, 22.0);
    let mut hit_target = sample_hit_target(false, false, false, false);

    assert_eq!(
        hit_target.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 10.0)),),
        None
    );

    let plan = hit_target.paint_plan_with_defaults(bounds);
    let fills = plan.fill_rects().collect::<Vec<_>>();
    let hover_fill = shared_dense_row_palette()
        .hovered
        .expect("dense-row hover fill");

    assert!(
        fills
            .iter()
            .any(|fill| fill.rect == bounds && fill.color == hover_fill),
        "{fills:?}"
    );
}

#[test]
fn full_gui_frame_places_sample_browser_text_inside_visible_area() {
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let expected_names = state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    let frame = crate::native_app::test_support::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(1517.0, 758.0));
    let sample_texts = frame
        .paint_plan
        .text_runs()
        .filter(|text| {
            text.text.as_str() == "Name"
                || expected_names
                    .iter()
                    .any(|name| text.text.as_str().starts_with(name))
        })
        .map(|text| (text.text.as_str().to_string(), text.rect, text.baseline))
        .collect::<Vec<_>>();

    assert!(!sample_texts.is_empty(), "{sample_texts:?}");
    assert!(
        sample_texts.iter().any(|(_, rect, baseline)| {
            rect.width() > 20.0
                && rect.height() >= 10.0
                && rect.min.x >= 280.0
                && rect.min.y >= 320.0
                && rect.max.y <= 730.0
                && baseline.is_some()
        }),
        "{sample_texts:?}"
    );
}
