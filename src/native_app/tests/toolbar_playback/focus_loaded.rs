use super::*;

#[test]
fn focus_loaded_toolbar_button_is_topmost_hit_target_and_paints_hover_feedback() {
    let state = NativeAppState::load_default().expect("default state loads");
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(
            crate::native_app::test_support::toolbar::TOOLBAR_FOCUS_LOADED_ID,
        )
        .expect("focus-loaded toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_FOCUS_LOADED_ID),
        "focus-loaded button must be the topmost hit target at its painted icon"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_FOCUS_LOADED_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::toolbar::TOOLBAR_FOCUS_LOADED_ID
            ),
        "hovering the focus-loaded button should paint a visible accent overlay"
    );
}

#[test]
fn focus_loaded_action_scrolls_loaded_sample_into_file_view() {
    let root = temp_gui_root("wavecrate-toolbar-focus-loaded-scroll");
    let files = (0..140)
        .map(|index| root.join(format!("sample_{index:03}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write sample");
    }
    let loaded = files[130].clone();
    let loaded_id = loaded.display().to_string();
    write_test_wav_i16(&loaded, &[0, 1024, -1024, 512]);
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(loaded.clone())
            .expect("load sample");
    state
        .library
        .folder_browser
        .select_file(files[0].display().to_string());
    state
        .library
        .folder_browser
        .follow_selected_file_view(16, 1, 1);
    assert_eq!(state.library.folder_browser.file_view_start(), 0);

    state.focus_loaded_file(&mut ui::UiUpdateContext::default());
    state
        .library
        .folder_browser
        .follow_selected_file_view(16, 1, 1);

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(loaded_id.as_str())
    );
    assert!(
        state.library.folder_browser.file_view_start() > 0,
        "focusing loaded sample should move the retained file viewport to the loaded row"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn focus_loaded_action_preserves_recursive_folder_when_loaded_sample_is_visible() {
    let root = temp_gui_root("wavecrate-toolbar-focus-loaded-recursive");
    let nested = root.join("nested");
    fs::create_dir_all(&nested).expect("create nested folder");
    let files = (0..140)
        .map(|index| nested.join(format!("sample_{index:03}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write sample");
    }
    let loaded = files[130].clone();
    let loaded_id = loaded.display().to_string();
    write_test_wav_i16(&loaded, &[0, 1024, -1024, 512]);
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.library.folder_browser.toggle_folder_subtree_listing();
    state
        .library
        .folder_browser
        .select_file(files[0].display().to_string());
    state
        .library
        .folder_browser
        .follow_selected_file_view(16, 1, 1);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(loaded.clone())
            .expect("load sample");

    state.focus_loaded_file(&mut ui::UiUpdateContext::default());
    state
        .library
        .folder_browser
        .follow_selected_file_view(16, 1, 1);
    let root_id = root.display().to_string();

    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(root_id.as_str())
    );
    assert!(
        state
            .library
            .folder_browser
            .folder_subtree_listing_enabled()
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(loaded_id.as_str())
    );
    assert!(
        state.library.folder_browser.file_view_start() > 0,
        "focusing a loaded sample already visible through recursive listing should scroll the sample list without changing the folder"
    );

    let _ = fs::remove_dir_all(root);
}
