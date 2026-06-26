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
fn sample_browser_keyboard_scroll_context_matches_selection_follow() {
    assert_eq!(
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
        2
    );
    assert_eq!(
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS + 1
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
    let plan = sample_hit_target_plan(&widget, bounds);
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
fn copied_sample_browser_row_paints_copy_flash_fill() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("flash.wav");
    fs::write(&sample_path, []).expect("sample file");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path.display().to_string());

    let mut context = radiant::prelude::UiUpdateContext::default();
    state.copy_selected_files(&mut context);
    crate::native_app::test_support::sample_browser::prepare_sample_browser_view(&mut state);
    let frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));

    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == Rgba8::new(71, 220, 255, 118)),
        "copied sample rows should paint the transient copy flash fill"
    );
}

#[test]
fn sample_browser_row_hover_paints_bright_background_without_marker() {
    let bounds = Rect::from_size(180.0, 22.0);
    let mut hit_target = sample_hit_target(false, false, false, false);

    assert_eq!(
        sample_hit_target_input(
            &mut hit_target,
            bounds,
            WidgetInput::pointer_move(Point::new(20.0, 10.0)),
        ),
        None
    );

    let plan = sample_hit_target_plan(&hit_target, bounds);
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
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    for name in ["visible_kick.wav", "visible_snare.wav"] {
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
        .filter(|text| {
            text.text.starts_with("scroll_sample_")
                && list_rect.contains(Point::new(text.rect.center().x, text.rect.center().y))
        })
        .collect::<Vec<_>>();
    let expected_visible_rows = (list_rect.height()
        / crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT)
        .floor()
        .max(1.0) as usize;
    let mut row_tops = rendered_samples
        .iter()
        .map(|text| text.rect.min.y)
        .collect::<Vec<_>>();
    row_tops.sort_by(|left, right| left.total_cmp(right));
    row_tops.dedup_by(|left, right| (*left - *right).abs() < 0.5);

    assert!(
        rendered_samples.len() >= expected_visible_rows.saturating_sub(2),
        "fast scrolling should keep visible sample rows materialized, got {:?}",
        frame.paint_plan.text_label_strings()
    );
    assert!(
        row_tops.windows(2).all(|pair| {
            (pair[1] - pair[0])
                <= crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT * 1.5
        }),
        "fast scrolling should not leave blank row-height gaps: {row_tops:?}"
    );
}

#[test]
fn full_gui_random_navigation_to_row_above_keeps_recursive_rows_rendered() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    for index in 0..180 {
        let folder = source_root.path().join(format!("group_{:02}", index / 12));
        fs::create_dir_all(&folder).expect("create sample folder");
        write_test_wav_i16(
            &folder.join(format!("random_recursive_{index:03}.wav")),
            &[0, 128, -128, 64],
        );
    }
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.toggle_folder_subtree_listing();

    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let list_rect = runtime
        .layout()
        .rects
        .get(&crate::native_app::ui::ids::SAMPLE_BROWSER_LIST_ID)
        .copied()
        .expect("sample browser list should be laid out");
    let ids = runtime
        .bridge()
        .state()
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.id.clone())
        .collect::<Vec<_>>();
    let current_index = 150;
    let target_index = 5;
    let current_id = ids[current_index].clone();
    let target_id = ids[target_index].clone();
    let target_stem = PathBuf::from(&target_id)
        .file_stem()
        .expect("target file stem")
        .to_string_lossy()
        .to_string();

    runtime.execute_command(Command::scroll_fixed_row_into_view(
        crate::native_app::ui::ids::SAMPLE_BROWSER_LIST_ID,
        current_index,
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT,
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
        crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
        0,
    ));
    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: current_id.clone(),
            modifiers: PointerModifiers::default(),
        },
    );
    let visited = ids
        .iter()
        .filter(|id| *id != &target_id)
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    runtime
        .bridge_mut()
        .state_mut()
        .library
        .folder_browser
        .seed_random_navigation_for_tests(ids, visited, vec![current_id]);

    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
        },
    );

    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(target_id.as_str())
    );
    let frame = runtime.frame_with_default_theme();
    let rendered_target = frame.paint_plan.text_runs().any(|text| {
        text.text.as_str() == target_stem
            && list_rect.contains(Point::new(text.rect.center().x, text.rect.center().y))
    });
    assert!(
        rendered_target,
        "random navigation to an earlier recursive row should keep the target materialized; painted labels were {:?}",
        frame.paint_plan.text_label_strings()
    );

    let _ = fs::remove_dir_all(source_root);
}

#[test]
fn full_gui_bottom_keyboard_navigation_keeps_sample_window_stable() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    let sample_paths = (0..160)
        .map(|index| {
            let path = source_root
                .path()
                .join(format!("bottom_nav_sample_{index:03}.wav"));
            std::fs::write(&path, []).expect("sample file");
            path
        })
        .collect::<Vec<_>>();
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
    for _ in 0..64 {
        assert!(runtime.scroll_at(scroll_point, Vector2::new(0.0, 110.0)));
    }

    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: sample_paths[150].display().to_string(),
            modifiers: PointerModifiers::default(),
        },
    );

    let mut starts = vec![
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .file_view_start(),
    ];
    for _ in 0..14 {
        runtime.dispatch_message(
            crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
                delta: 1,
                extend: false,
                preserve_selection: false,
            },
        );
        starts.push(
            runtime
                .bridge()
                .state()
                .library
                .folder_browser
                .file_view_start(),
        );
    }

    assert!(
        starts.windows(2).all(|pair| pair[1] >= pair[0]),
        "bottom navigation should not bounce the sample window: {starts:?}"
    );
    let selected = runtime
        .bridge()
        .state()
        .library
        .folder_browser
        .selected_file_id()
        .expect("sample selection should remain active");
    assert_eq!(selected, sample_paths[159].display().to_string());
    let last = *starts.last().expect("at least one window start");
    for _ in 0..3 {
        runtime.dispatch_message(
            crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
                delta: 1,
                extend: false,
                preserve_selection: false,
            },
        );
        assert_eq!(
            runtime
                .bridge()
                .state()
                .library
                .folder_browser
                .file_view_start(),
            last,
            "edge navigation at the bottom should leave the sample window stable"
        );
    }
}

#[test]
fn full_gui_bottom_pointer_selection_does_not_jump_sample_window() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    let sample_paths = (0..160)
        .map(|index| {
            let path = source_root
                .path()
                .join(format!("bottom_click_sample_{index:03}.wav"));
            std::fs::write(&path, []).expect("sample file");
            path
        })
        .collect::<Vec<_>>();
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
    for _ in 0..64 {
        assert!(runtime.scroll_at(scroll_point, Vector2::new(0.0, 110.0)));
    }

    let before = runtime
        .bridge()
        .state()
        .library
        .folder_browser
        .file_view_start();
    let viewport_rows = (list_rect.height()
        / crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT)
        .ceil()
        .max(1.0) as usize;
    let bottom_visible_row = (before + viewport_rows.saturating_sub(1)).min(sample_paths.len() - 1);
    let bottom_visible_stem = sample_paths[bottom_visible_row]
        .file_stem()
        .expect("sample file stem")
        .to_string_lossy()
        .to_string();
    let frame = runtime.frame_with_default_theme();

    runtime.dispatch_primary_click(text_center(&frame, &bottom_visible_stem));
    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(
            sample_paths[bottom_visible_row]
                .display()
                .to_string()
                .as_str()
        ),
        "bottom row click should select the intended sample"
    );

    let mut starts = vec![
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .file_view_start(),
    ];
    let mut selected_row_tops = vec![text_top(
        &runtime.frame_with_default_theme(),
        &bottom_visible_stem,
    )];
    for _ in 0..4 {
        runtime.refresh();
        starts.push(
            runtime
                .bridge()
                .state()
                .library
                .folder_browser
                .file_view_start(),
        );
        selected_row_tops.push(text_top(
            &runtime.frame_with_default_theme(),
            &bottom_visible_stem,
        ));
    }

    assert_eq!(
        starts,
        vec![before; starts.len()],
        "clicking a bottom-visible row should not move an already settled bottom viewport"
    );
    assert!(
        selected_row_tops
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.5),
        "clicking a bottom-visible row should not repaint it at a different y position: {selected_row_tops:?}"
    );
}

#[test]
fn full_gui_bottom_row_hover_does_not_shift_sample_window() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    let sample_paths = (0..160)
        .map(|index| {
            let path = source_root
                .path()
                .join(format!("bottom_hover_sample_{index:03}.wav"));
            std::fs::write(&path, []).expect("sample file");
            path
        })
        .collect::<Vec<_>>();
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
    for _ in 0..64 {
        assert!(runtime.scroll_at(scroll_point, Vector2::new(0.0, 110.0)));
    }

    let before = runtime
        .bridge()
        .state()
        .library
        .folder_browser
        .file_view_start();
    let viewport_rows = (list_rect.height()
        / crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT)
        .ceil()
        .max(1.0) as usize;
    let bottom_visible_row = (before + viewport_rows.saturating_sub(1)).min(sample_paths.len() - 1);
    let first_hover_row = bottom_visible_row.saturating_sub(3).max(before);
    let hover_stems = (first_hover_row..=bottom_visible_row)
        .map(|index| {
            sample_paths[index]
                .file_stem()
                .expect("sample file stem")
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();
    let tracked_stem = hover_stems
        .last()
        .expect("at least one hover row")
        .to_string();

    let mut starts = vec![before];
    let mut tracked_row_tops = vec![text_top(&runtime.frame_with_default_theme(), &tracked_stem)];
    for _ in 0..3 {
        for stem in &hover_stems {
            let frame = runtime.frame_with_default_theme();
            let hover_target = text_center(&frame, stem);
            assert!(
                runtime
                    .dispatch_event(Event::pointer_move(hover_target))
                    .is_some(),
                "sample row should receive pointer hover"
            );
            starts.push(
                runtime
                    .bridge()
                    .state()
                    .library
                    .folder_browser
                    .file_view_start(),
            );
            tracked_row_tops.push(text_top(&runtime.frame_with_default_theme(), &tracked_stem));
            runtime.refresh();
            starts.push(
                runtime
                    .bridge()
                    .state()
                    .library
                    .folder_browser
                    .file_view_start(),
            );
            tracked_row_tops.push(text_top(&runtime.frame_with_default_theme(), &tracked_stem));
        }
    }

    assert_eq!(
        starts,
        vec![before; starts.len()],
        "hovering bottom-visible rows should not move an already settled bottom viewport"
    );
    assert!(
        tracked_row_tops
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.5),
        "hovering bottom-visible rows should not repaint them at different y positions: {tracked_row_tops:?}"
    );
}

#[test]
fn full_gui_lower_pointer_selection_preserves_manual_scroll_window() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let source_root = tempfile::tempdir().expect("source root");
    let sample_paths = (0..160)
        .map(|index| {
            let path = source_root
                .path()
                .join(format!("lower_click_sample_{index:03}.wav"));
            std::fs::write(&path, []).expect("sample file");
            path
        })
        .collect::<Vec<_>>();
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
    for _ in 0..64 {
        assert!(runtime.scroll_at(scroll_point, Vector2::new(0.0, 110.0)));
    }

    let before = runtime
        .bridge()
        .state()
        .library
        .folder_browser
        .file_view_start();
    let viewport_rows = (list_rect.height()
        / crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_ROW_HEIGHT)
        .ceil()
        .max(1.0) as usize;
    let lower_visible_row = (before
        + viewport_rows.saturating_sub(
            crate::native_app::test_support::sample_browser::SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
        ))
    .min(sample_paths.len() - 1);
    let lower_visible_stem = sample_paths[lower_visible_row]
        .file_stem()
        .expect("sample file stem")
        .to_string_lossy()
        .to_string();
    let frame = runtime.frame_with_default_theme();

    runtime.dispatch_primary_click(text_center(&frame, &lower_visible_stem));
    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(
            sample_paths[lower_visible_row]
                .display()
                .to_string()
                .as_str()
        ),
        "lower visible row click should select the intended sample"
    );

    let mut starts = vec![
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .file_view_start(),
    ];
    let mut selected_row_tops = vec![text_top(
        &runtime.frame_with_default_theme(),
        &lower_visible_stem,
    )];
    for _ in 0..4 {
        runtime.refresh();
        starts.push(
            runtime
                .bridge()
                .state()
                .library
                .folder_browser
                .file_view_start(),
        );
        selected_row_tops.push(text_top(
            &runtime.frame_with_default_theme(),
            &lower_visible_stem,
        ));
    }

    assert_eq!(
        starts,
        vec![before; starts.len()],
        "clicking a lower visible row after manual scroll should preserve the viewport"
    );
    assert!(
        selected_row_tops
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.5),
        "clicking a lower visible row should not repaint it at a different y position: {selected_row_tops:?}"
    );
}

fn text_top(frame: &radiant::runtime::SurfaceFrame, label: &str) -> f32 {
    frame
        .paint_plan
        .text_runs()
        .find(|text| text.text.as_str() == label)
        .map(|text| text.rect.min.y)
        .unwrap_or_else(|| panic!("{label} should paint"))
}
