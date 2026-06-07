use super::*;
use crate::native_app::app_chrome::toolbar::{MainToolbarViewModel, main_toolbar};

#[test]
fn toolbar_icon_assets_parse_and_paint_through_radiant_icon_button() {
    for icon in [
        crate::native_app::test_support::ToolbarIcon::FocusLoaded,
        crate::native_app::test_support::ToolbarIcon::Loop,
        crate::native_app::test_support::ToolbarIcon::Random,
        crate::native_app::test_support::ToolbarIcon::Play,
        crate::native_app::test_support::ToolbarIcon::Stop,
    ] {
        assert_eq!(
            crate::native_app::test_support::toolbar_icon_color(true, false),
            radiant::prelude::Rgba8::new(238, 238, 238, 255)
        );
        assert_eq!(
            crate::native_app::test_support::toolbar_icon_color(true, true),
            radiant::prelude::Rgba8::new(255, 160, 82, 255)
        );
        assert_eq!(
            crate::native_app::test_support::toolbar_icon_color(false, false),
            radiant::prelude::Rgba8::new(145, 145, 145, 255)
        );
        let mut primitives = Vec::new();
        crate::native_app::test_support::toolbar_icon_glyph(icon, true, false).append_paint(
            &mut primitives,
            101,
            Rect::from_size(28.0, 24.0),
        );
        assert!(
            primitives.iter().any(|primitive| primitive.svg().is_some()),
            "toolbar icon cache should produce a retained Radiant SVG"
        );
        let frame = crate::native_app::test_support::toolbar_icon_button(101, icon, true, false)
            .view_frame_at_size_with_default_theme(Vector2::new(28.0, 24.0));
        assert!(
            frame.paint_plan.svgs().next().is_some(),
            "toolbar icon should paint as a retained Radiant SVG"
        );
    }
}

#[test]
fn toolbar_icon_button_routes_messages_through_radiant_builder() {
    for (icon, message) in [
        (
            crate::native_app::test_support::ToolbarIcon::FocusLoaded,
            crate::native_app::test_support::GuiMessage::FocusLoadedFile,
        ),
        (
            crate::native_app::test_support::ToolbarIcon::Loop,
            crate::native_app::test_support::GuiMessage::ToggleLoopPlayback,
        ),
        (
            crate::native_app::test_support::ToolbarIcon::Random,
            crate::native_app::test_support::GuiMessage::PlayRandomSampleRange,
        ),
    ] {
        assert_eq!(
            crate::native_app::test_support::toolbar_icon_button(101, icon, true, false)
                .view_dispatch_widget_output(
                    101,
                    radiant::widgets::WidgetOutput::typed(
                        radiant::widgets::ButtonMessage::Activate
                    ),
                ),
            Some(message)
        );
    }
}

#[test]
fn main_toolbar_does_not_paint_empty_spacer_border() {
    let state = NativeAppState::load_default().expect("default state loads");
    let frame = main_toolbar(MainToolbarViewModel::from_app_state(&state))
        .view_frame_at_size_with_default_theme(Vector2::new(664.0, 34.0));

    assert!(
        !frame
            .paint_plan
            .contains_paint_rect_matching(|rect| rect.width() > 100.0 && rect.height() >= 20.0),
        "empty toolbar spacer should not paint or reserve a large visible rectangle"
    );
}

#[test]
fn main_toolbar_view_model_projects_playback_state() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    let empty = MainToolbarViewModel::from_app_state(&state);
    assert_eq!(empty.random_available, state.random_playback_available());
    assert!(!empty.loop_playback);
    assert!(!empty.playing);

    state.loop_playback = true;
    state.waveform = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    state.waveform.start_playback(0.25);

    let loaded = MainToolbarViewModel::from_app_state(&state);
    assert_eq!(loaded.random_available, state.random_playback_available());
    assert!(loaded.loop_playback);
    assert!(loaded.playing);
}

#[test]
fn random_toolbar_button_is_hit_target_for_loaded_sample() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_RANDOM_ID),
        "random button must be the topmost hit target for loaded samples"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
    );
}

#[test]
fn random_toolbar_button_is_hit_target_for_unselected_browser_sample() {
    let root = temp_gui_root("wavecrate-toolbar-random-unselected");
    let sample = root.join("unselected.wav");
    fs::write(&sample, []).expect("write sample");
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    assert!(!state.waveform.has_loaded_sample());
    assert_eq!(state.folder_browser.selected_file_id(), None);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::TOOLBAR_RANDOM_ID
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
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    assert!(!state.waveform.has_loaded_sample());
    assert_eq!(state.folder_browser.selected_file_id(), None);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");

    runtime.dispatch_primary_click(icon_rect.center());

    assert_eq!(
        runtime.bridge().state().folder_browser.selected_file_id(),
        Some(sample_id.as_str())
    );
    assert!(
        matches!(
            runtime.bridge().state().pending_sample_playback,
            Some(crate::native_app::test_support::PendingSamplePlayback::RandomAudition { .. })
        ),
        "random toolbar click should preserve random-audition intent while the browser sample loads"
    );
    assert!(
        runtime
            .bridge()
            .state()
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
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state
        .folder_browser
        .select_file(sample.display().to_string());
    assert!(!state.waveform.has_loaded_sample());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::TOOLBAR_RANDOM_ID
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
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state
        .folder_browser
        .select_file(sample.display().to_string());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_RANDOM_ID)
        .expect("random toolbar icon should paint");

    runtime.dispatch_primary_click(icon_rect.center());

    assert!(
        matches!(
            runtime.bridge().state().pending_sample_playback,
            Some(crate::native_app::test_support::PendingSamplePlayback::RandomAudition { .. })
        ),
        "random toolbar click should preserve random-audition intent while the selected sample loads"
    );
    assert!(
        runtime
            .bridge()
            .state()
            .deferred_sample_load_task
            .active()
            .is_some(),
        "selected unloaded sample should queue the normal debounced load"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn focus_loaded_toolbar_button_is_topmost_hit_target_and_paints_hover_feedback() {
    let state = NativeAppState::load_default().expect("default state loads");
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_FOCUS_LOADED_ID)
        .expect("focus-loaded toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_FOCUS_LOADED_ID),
        "focus-loaded button must be the topmost hit target at its painted icon"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_FOCUS_LOADED_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::TOOLBAR_FOCUS_LOADED_ID
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
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.waveform = crate::native_app::test_support::WaveformState::load_path(loaded.clone())
        .expect("load sample");
    state
        .folder_browser
        .select_file(files[0].display().to_string());
    state.folder_browser.follow_selected_file_view(16, 1, 1);
    assert_eq!(state.folder_browser.file_view_start(), 0);

    state.focus_loaded_file(&mut ui::UpdateContext::default());
    state.folder_browser.follow_selected_file_view(16, 1, 1);

    assert_eq!(
        state.folder_browser.selected_file_id(),
        Some(loaded_id.as_str())
    );
    assert!(
        state.folder_browser.file_view_start() > 0,
        "focusing loaded sample should move the retained file viewport to the loaded row"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn stop_toolbar_button_is_hit_target_and_paints_hover_while_playing() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    state.waveform.start_playback(0.25);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID),
        "stop button must be the topmost hit target while playback is active"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::TOOLBAR_STOP_ID
            ),
        "hovering the playing stop button should paint a visible accent overlay"
    );
    runtime.dispatch_primary_click(point);
    assert!(
        !runtime.bridge().state().waveform.is_playing(),
        "clicking the playing stop button should dispatch StopPlayback"
    );
}

#[test]
fn stop_toolbar_button_remains_available_for_loaded_idle_sample() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    assert!(!state.waveform.is_playing());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
}

#[test]
fn stop_toolbar_button_remains_hit_target_without_loaded_sample() {
    let state = NativeAppState::load_default().expect("default state loads");
    assert!(!state.waveform.has_loaded_sample());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::TOOLBAR_STOP_ID
            ),
        "hovering stop should paint feedback even before a waveform is loaded"
    );
}

#[test]
fn playback_frame_uses_paint_only_when_only_playhead_changes() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);

    let before = state.frame_repaint_scope_before_update();
    state.advance_frame();

    assert!(
        state.frame_can_use_paint_only(before),
        "playback-only frames should not force full surface reprojection"
    );
}

#[test]
fn playback_frame_repaints_surface_when_playback_state_changes() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);

    let before = state.frame_repaint_scope_before_update();
    state.waveform.stop_playback();

    assert!(
        !state.frame_can_use_paint_only(before),
        "stopping playback changes toolbar/status surface state and needs a full repaint"
    );
}

#[test]
fn frame_animation_stays_active_for_pending_startup_source_scan() {
    let mut state = gui_state_for_span_tests();
    assert!(!state.frame_message_animation_active());

    state.startup_source_scan_pending = true;

    assert!(
        state.frame_message_animation_active(),
        "startup source restoration needs a frame message to queue the source scan"
    );
}

#[test]
fn frame_animation_stays_active_for_pending_startup_auto_load() {
    let mut state = gui_state_for_span_tests();
    assert!(!state.frame_message_animation_active());

    state.startup_auto_load_pending = true;

    assert!(
        state.frame_message_animation_active(),
        "startup sample auto-load needs frame messages until the restored source is loaded"
    );
}

#[test]
fn playback_cursor_paints_as_transient_overlay() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);

    assert!(
        !frame
            .paint_plan
            .fill_rects_for_widget(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
            .any(|fill| { fill.color.r == 71 && fill.color.g == 220 && fill.color.b == 255 }),
        "live playback cursor should not be baked into the cached surface"
    );

    let mut primitives = Vec::new();
    runtime.bridge_mut().state_mut().paint_playback_overlay(
        TransientOverlayContext::new(
            &frame.paint_plan,
            Vector2::new(900.0, 620.0),
            Duration::ZERO,
        ),
        &mut primitives,
    );

    assert!(
        primitives
            .iter()
            .filter_map(|primitive| primitive.fill_rect())
            .any(|fill| {
                fill.widget_id == crate::native_app::test_support::WAVEFORM_WIDGET_ID
                    && fill.color.r == 71
                    && fill.color.g == 220
                    && fill.color.b == 255
            }),
        "paint-only playback overlay should append the live cursor"
    );
}
