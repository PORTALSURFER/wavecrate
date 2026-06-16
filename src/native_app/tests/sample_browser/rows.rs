use super::*;

fn shared_dense_row_palette() -> radiant::prelude::DenseRowPalette {
    radiant::prelude::dense_row_palette_from_style(
        &radiant::prelude::ThemeTokens::default(),
        radiant::prelude::WidgetStyle::subtle(radiant::prelude::WidgetTone::Accent),
    )
}

#[test]
fn sample_browser_rows_match_keyboard_scroll_stride() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    for name in ["alpha.wav", "beta.wav", "gamma.wav"] {
        std::fs::write(source_root.path().join(name), []).expect("sample file");
    }
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let expected_names = state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    crate::native_app::test_support::sample_browser::prepare_sample_browser_view(&mut state);
    let frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
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
            ((pair[1] - pair[0])
                - crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT)
                .abs()
                < 0.5
        }),
        "{row_tops:?}"
    );
}

#[test]
fn sample_browser_projection_window_matches_rendered_row_order() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let projection_names = {
        crate::native_app::test_support::sample_browser::prepare_sample_browser_view(&mut state);
        let projection =
            crate::native_app::test_support::sample_browser::sample_browser_window_projection(
                &state, 4,
            );
        assert_eq!(projection.total_items, projection.total_count);
        assert_eq!(projection.visible_rows, projection.window_len);
        projection.first_stems
    };
    crate::native_app::test_support::sample_browser::prepare_sample_browser_view(&mut state);
    let frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
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
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
        2
    );
    assert_eq!(
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT,
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
fn full_gui_sample_row_hover_survives_surface_refresh() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let kick = source_root.path().join("kick.wav");
    let snare = source_root.path().join("snare.wav");
    fs::write(&kick, []).expect("write kick");
    fs::write(&snare, []).expect("write snare");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(kick.display().to_string());

    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let snare_target = text_center(&frame, "snare");
    assert!(
        runtime
            .dispatch_event(Event::pointer_move(snare_target))
            .is_some(),
        "sample row should receive pointer hover"
    );
    let hovered_widget = runtime.hovered_widget();
    assert!(hovered_widget.is_some(), "sample row should own hover");

    runtime.refresh();

    assert_eq!(
        runtime.hovered_widget(),
        hovered_widget,
        "surface refresh should preserve the current sample-row hover owner"
    );
    let refreshed_frame = runtime.frame_with_default_theme();
    let hover_fill = shared_dense_row_palette()
        .hovered
        .expect("dense-row hover fill");
    assert!(
        refreshed_frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == hover_fill),
        "hovered sample row should keep its visible hover fill after refresh"
    );
}

#[test]
fn full_gui_frame_places_sample_browser_text_inside_visible_area() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let expected_names = state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    let frame = crate::native_app::test_support::state::view(&mut state)
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

#[test]
fn full_gui_fast_sample_browser_scroll_keeps_rows_rendered() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    for index in 0..320 {
        std::fs::write(
            source_root
                .path()
                .join(format!("scroll_sample_{index:03}.wav")),
            [],
        )
        .expect("sample file");
    }
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);

    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let list_rect = runtime
        .layout()
        .rects
        .get(&crate::native_app::ui::ids::SAMPLE_BROWSER_LIST_ID)
        .copied()
        .expect("sample browser list should be laid out");
    let scroll_point = Point::new(list_rect.center().x, list_rect.min.y + 48.0);

    for _ in 0..48 {
        assert!(
            runtime.scroll_at(scroll_point, Vector2::new(0.0, 66.0)),
            "sample browser should accept repeated scroll input"
        );
    }

    let frame = runtime.frame_with_default_theme();
    let rendered_samples = frame
        .paint_plan
        .text_runs()
        .filter(|text| text.text.starts_with("scroll_sample_"))
        .collect::<Vec<_>>();

    assert!(
        rendered_samples.len() >= 8,
        "fast scrolling should keep materialized sample rows visible, got {:?}",
        frame.paint_plan.text_label_strings()
    );
}
