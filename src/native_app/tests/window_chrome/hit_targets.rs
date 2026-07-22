use super::*;

#[test]
fn full_app_scene_routes_waveform_hit_target() {
    let state = gui_state_for_span_tests();
    let runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let point = waveform_rect(&runtime).center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
}

#[test]
fn absent_playmark_layers_leave_all_waveform_pointer_gestures_available() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let press = Point::new(rect.min.x + rect.width() * 0.2, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.4, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(press)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.hovered_widget(),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(press)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(drag)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.4))
    );

    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(press)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(drag)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.edit_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.4))
    );
}

#[test]
fn playmark_local_controls_leave_waveform_input_outside_their_painted_bounds() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.25, 0.75);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let theme = radiant::theme::ThemeTokens::default();
    let _ = runtime.frame(&theme);
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.1, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.hovered_widget(),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        Some(0.1)
    );
}

#[test]
fn playmark_time_label_claims_edit_click_but_leaves_hover_and_secondary_drag_to_waveform() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.25, 0.75);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let theme = radiant::theme::ThemeTokens::default();
    let label_point = runtime
        .frame(&theme)
        .paint_plan
        .first_text_run("500 ms")
        .map(|text| text.rect.center())
        .expect("playmark time label paint");

    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(label_point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID),
        "the time label must leave waveform hover and transient overlays active"
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(label_point)),
        Some(crate::native_app::ui::ids::WAVEFORM_PLAYMARK_LABEL_ID),
        "a normal click must start editing the time label"
    );
    assert!(
        runtime
            .bridge()
            .state()
            .waveform
            .current
            .playmark_label_editor_active()
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(label_point)),
        Some(crate::native_app::ui::ids::WAVEFORM_PLAYMARK_LABEL_ID)
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.75))
    );

    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.25, 0.75);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let label_point = frame
        .paint_plan
        .first_text_run("500 ms")
        .map(|text| text.rect.center())
        .expect("playmark time label paint");
    let drag_point = Point::new(waveform_rect(&runtime).max.x - 20.0, label_point.y);
    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(label_point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID),
        "an edit-selection drag may start on the time label"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag_point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(drag_point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime
            .bridge()
            .state()
            .waveform
            .current
            .edit_selection()
            .is_some()
    );
}

#[test]
fn playmark_local_beat_controls_consume_hits_and_update_shared_state() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.25, 0.75);
    let original_selection = state.waveform.current.play_selection();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let theme = radiant::theme::ThemeTokens::default();
    let frame = runtime.frame(&theme);
    let toggle_id = crate::native_app::ui::ids::WAVEFORM_PLAYMARK_BEAT_TOGGLE_ID;
    let toggle_point = frame
        .paint_plan
        .text_runs()
        .find(|text| text.widget_id == toggle_id && text.text.as_str() == "Grid")
        .map(|text| text.rect)
        .expect("local beat toggle paint")
        .center();
    let count_id = crate::native_app::ui::ids::WAVEFORM_PLAYMARK_BEAT_COUNT_ID;
    assert!(
        runtime.surface().find_widget(count_id).is_none(),
        "the beat count stays hidden until the grid is enabled"
    );
    let toggle_press = WidgetInput::PointerPress {
        position: toggle_point,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    };

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(toggle_point)),
        Some(toggle_id)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(toggle_point)),
        Some(toggle_id)
    );
    assert!(runtime.bridge().state().ui.chrome.beat_guides_enabled);
    assert!(
        crate::native_app::test_support::toolbar::main_toolbar_projection(runtime.bridge().state())
            .beat_guides_enabled
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_selection(),
        original_selection,
        "local toggle must not leak into waveform gestures"
    );

    let frame = runtime.frame(&theme);
    assert!(frame.paint_plan.contains_text("480 BPM"));
    assert!(!frame.paint_plan.contains_text("500 ms"));
    assert!(
        !runtime
            .surface()
            .find_widget(count_id)
            .expect("count widget")
            .widget_object()
            .accepts_pointer_input(&toggle_press),
        "count widget must reject the adjacent toggle hit"
    );
    let count_point = frame
        .paint_plan
        .text_inputs()
        .find(|input| input.widget_id == count_id)
        .map(|input| input.rect)
        .expect("local beat count paint")
        .center();
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(count_point)),
        Some(count_id)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(count_point)),
        Some(count_id)
    );
    assert_eq!(
        runtime.dispatch_event(Event::KeyPress(WidgetKey::ArrowUp)),
        Some(count_id)
    );
    assert_eq!(runtime.bridge().state().ui.chrome.beat_guide_count, 5);
    assert_eq!(
        crate::native_app::test_support::toolbar::main_toolbar_projection(runtime.bridge().state())
            .beat_guide_count,
        5
    );
    assert!(runtime.frame(&theme).paint_plan.contains_text("600 BPM"));
    assert_eq!(
        runtime.bridge().state().waveform.current.play_selection(),
        original_selection,
        "local count editing must not leak into waveform gestures"
    );

    runtime.clear_focus();
    assert!(runtime.wheel_or_scroll_at(count_point, Vector2::new(0.0, -40.0)));
    assert_eq!(runtime.bridge().state().ui.chrome.beat_guide_count, 6);
    assert!(runtime.frame(&theme).paint_plan.contains_text("720 BPM"));

    let horizontal_wheel = WidgetInput::Wheel {
        position: count_point,
        delta: Vector2::new(40.0, 0.0),
        modifiers: Default::default(),
    };
    assert!(
        !runtime
            .surface()
            .find_widget(count_id)
            .expect("count widget")
            .widget_object()
            .accepts_pointer_input(&horizontal_wheel),
        "the local count field must leave horizontal wheel input to the waveform"
    );
    assert!(runtime.wheel_or_scroll_at(count_point, Vector2::new(40.0, 0.0)));
    assert_eq!(
        runtime.bridge().state().ui.chrome.beat_guide_count,
        6,
        "horizontal wheel fallback must not change the beat count"
    );

    let waveform_wheel_point = Point::new(
        waveform_rect(&runtime).min.x + 12.0,
        waveform_rect(&runtime).center().y,
    );
    assert!(runtime.wheel_or_scroll_at(waveform_wheel_point, Vector2::new(0.0, -40.0)));
    assert_eq!(
        runtime.bridge().state().ui.chrome.beat_guide_count,
        6,
        "the full-size playmark overlay must not steal wheel input outside its painted field"
    );
}

#[test]
fn stale_waveform_loading_label_does_not_mask_waveform_hit_target() {
    let mut state = gui_state_for_span_tests();
    state.waveform.load.label = Some(String::from("previous.wav"));
    state.waveform.load.progress = 0.5;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        None
    );
}

#[test]
fn stale_waveform_drop_hover_does_not_mask_waveform_hit_target() {
    let mut state = gui_state_for_span_tests();
    state.ui.browser_interaction.native_file_drop_hover = Some(
        crate::native_app::test_support::state::NativeFileDropHover {
            path: PathBuf::from("stale.wav"),
            supported: true,
        },
    );
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.38, rect.center().y);

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        runtime.bridge().state().waveform.current.edit_mark_ratio(),
        None
    );
}

#[test]
fn active_waveform_sample_load_masks_waveform_hit_target() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("blocking-load.wav");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: selected_file,
            modifiers: Default::default(),
        },
        &mut ui::UiUpdateContext::default(),
    );
    assert!(state.waveform_input_blocked_by_sample_load());
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_ne!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        None
    );
}
