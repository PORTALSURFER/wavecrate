use super::*;

#[test]
fn random_toolbar_button_is_hit_target_for_loaded_sample() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::synthetic_for_tests();
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID),
        "random button must be the topmost hit target for loaded samples"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
    );
}

#[test]
fn random_toolbar_button_is_hit_target_for_unselected_browser_sample() {
    let root = temp_gui_root("wavecrate-toolbar-random-unselected");
    let sample = root.join("unselected.wav");
    fs::write(&sample, []).expect("write sample");
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    assert!(!state.waveform.current.has_loaded_sample());
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID
            ),
        "hovering random with an available browser sample should paint feedback"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn random_toolbar_click_queues_random_audition_for_unselected_browser_sample() {
    let root = temp_gui_root("wavecrate-toolbar-random-click-unselected");
    let sample = root.join("unselected.wav");
    let sample_id = sample.display().to_string();
    fs::write(&sample, []).expect("write sample");
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    assert!(!state.waveform.current.has_loaded_sample());
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");

    runtime.dispatch_primary_click(icon_rect.center());

    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(sample_id.as_str())
    );
    assert!(
        matches!(
            runtime.bridge().state().audio.pending_sample_playback,
            Some(
                crate::native_app::test_support::state::PendingSamplePlayback::RandomAudition { .. }
            )
        ),
        "random toolbar click should preserve random-audition intent while the browser sample loads"
    );
    assert!(
        runtime
            .bridge()
            .state()
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "unselected browser sample should queue the normal debounced load"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn random_toolbar_button_is_hit_target_for_selected_unloaded_sample() {
    let root = temp_gui_root("wavecrate-toolbar-random-selected");
    let sample = root.join("selected.wav");
    fs::write(&sample, []).expect("write sample");
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());
    assert!(!state.waveform.current.has_loaded_sample());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID
            ),
        "hovering random with a selected sample should paint feedback"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn random_toolbar_click_queues_random_audition_for_selected_unloaded_sample() {
    let root = temp_gui_root("wavecrate-toolbar-random-click-selected");
    let sample = root.join("selected.wav");
    fs::write(&sample, []).expect("write sample");
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");

    runtime.dispatch_primary_click(icon_rect.center());

    assert!(
        matches!(
            runtime.bridge().state().audio.pending_sample_playback,
            Some(
                crate::native_app::test_support::state::PendingSamplePlayback::RandomAudition { .. }
            )
        ),
        "random toolbar click should preserve random-audition intent while the selected sample loads"
    );
    assert!(
        runtime
            .bridge()
            .state()
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "selected unloaded sample should queue the normal debounced load"
    );

    let _ = fs::remove_dir_all(root);
}
