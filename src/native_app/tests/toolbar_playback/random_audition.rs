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
        runtime.bridge().state().waveform_sample_load_active(),
        "unselected browser sample should queue a sample load"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn command_clicking_random_toolbar_button_toggles_sticky_random_playback() {
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
    let command = PointerModifiers {
        command: true,
        ..Default::default()
    };

    runtime.dispatch_event(Event::pointer_press(point, PointerButton::Primary, command));
    runtime.dispatch_event(Event::pointer_release(
        point,
        PointerButton::Primary,
        command,
    ));

    assert!(
        runtime
            .bridge()
            .state()
            .ui
            .chrome
            .sticky_random_sample_range_playback
    );
    assert_eq!(
        runtime.bridge().state().ui.status.sample,
        "Sticky random playback on: Space plays random sample sections"
    );

    runtime.dispatch_event(Event::pointer_press(point, PointerButton::Primary, command));
    runtime.dispatch_event(Event::pointer_release(
        point,
        PointerButton::Primary,
        command,
    ));

    assert!(
        !runtime
            .bridge()
            .state()
            .ui
            .chrome
            .sticky_random_sample_range_playback
    );
}

#[test]
fn shift_clicking_random_toolbar_button_selects_random_listed_sample_before_audition() {
    let root = temp_gui_root("wavecrate-toolbar-random-shift-listed");
    let tagged = root.join("tagged.wav");
    let hidden = root.join("hidden.wav");
    let tagged_id = tagged.display().to_string();
    let hidden_id = hidden.display().to_string();
    write_test_wav_i16(&tagged, &[0, 256, -256, 512]);
    write_test_wav_i16(&hidden, &[0, 1024, -1024, 512]);
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::synthetic_for_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state
        .metadata
        .tags_by_file
        .insert(tagged_id.clone(), vec![String::from("Drum")]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::TagFilterInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("drum"),
            },
        ),
    );
    assert_eq!(state.library.folder_browser.selected_file_id(), None);
    assert!(state.waveform.current.has_loaded_sample());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::toolbar::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");
    let point = icon_rect.center();
    let shift = PointerModifiers {
        shift: true,
        ..Default::default()
    };

    runtime.dispatch_event(Event::pointer_press(point, PointerButton::Primary, shift));
    runtime.dispatch_event(Event::pointer_release(point, PointerButton::Primary, shift));

    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(tagged_id.as_str())
    );
    assert_ne!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(hidden_id.as_str()),
        "Shift-random must choose from the listed filtered files"
    );
    assert!(
        matches!(
            runtime.bridge().state().audio.pending_sample_playback,
            Some(
                crate::native_app::test_support::state::PendingSamplePlayback::RandomAudition { .. }
            )
        ),
        "Shift-random should preserve random-audition intent while the listed sample loads"
    );
    assert!(
        runtime.bridge().state().waveform_sample_load_active(),
        "Shift-random should queue a sample load for the randomly listed sample"
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
        runtime.bridge().state().waveform_sample_load_active(),
        "selected unloaded sample should queue a sample load"
    );

    let _ = fs::remove_dir_all(root);
}
