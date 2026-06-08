use super::*;
use crate::native_app::app_chrome::waveform_panel::waveform_loading_visual;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};
use std::{cell::RefCell, rc::Rc};

fn waveform_rect(runtime: &NativeRuntimeForTests) -> Rect {
    *runtime
        .layout()
        .rects
        .get(&crate::native_app::test_support::WAVEFORM_WIDGET_ID)
        .expect("full app shell should lay out waveform widget")
}

fn assert_ratio_near(actual: Option<f32>, expected: f32) {
    let actual = actual.expect("expected waveform ratio");
    assert!(
        (actual - expected).abs() <= f32::EPSILON * 8.0,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn default_window_title_marks_alpha_build() {
    assert_eq!(
        crate::native_app::test_support::DEFAULT_WINDOW_TITLE,
        "Wavecrate - alpha"
    );
}

#[test]
fn audio_settings_popover_opens_as_centered_floating_window() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio_settings_error = None;
    let frame = crate::native_app::test_support::audio_settings_popover(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(520.0, 380.0));
    assert!(frame.paint_plan.contains_text("Settings"));
    assert!(frame.paint_plan.contains_text("Audio Engine"));
    let backend_rect = frame
        .paint_plan
        .first_text_run("Backend")
        .map(|text| text.rect)
        .expect("audio settings backend label paints");

    assert!(
        (146.0..=170.0).contains(&backend_rect.min.x),
        "{backend_rect:?}"
    );
    assert!(
        (34.0..=52.0).contains(&backend_rect.min.y),
        "{backend_rect:?}"
    );
}

#[test]
fn audio_settings_window_does_not_add_full_height_panel_chrome() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio_settings_open = true;
    let frame = crate::native_app::test_support::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 540.0));
    let audio_panel_fills = frame
        .paint_plan
        .fill_rects_for_widget(0)
        .filter(|fill| {
            fill.rect.min.x >= 250.0 && fill.rect.max.x <= 710.0 && fill.rect.width() >= 300.0
        })
        .map(|fill| fill.rect)
        .collect::<Vec<_>>();

    assert!(
        audio_panel_fills.iter().all(|rect| rect.height()
            <= crate::native_app::test_support::AUDIO_SETTINGS_POPUP_HEIGHT + 1.0),
        "{audio_panel_fills:?}"
    );
}

#[test]
fn audio_settings_window_does_not_block_waveform_selection_messages() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_open = true;
    let mut context = ui::UpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.45,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::UpdateSelection {
                visible_ratio: 0.65,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::FinishSelection {
                visible_ratio: 0.65,
            },
        ),
        &mut context,
    );

    assert_eq!(state.waveform.play_mark_ratio(), Some(0.45));
    assert_eq!(
        state.waveform.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.45, 0.65))
    );
}

#[test]
fn full_app_scene_routes_waveform_hit_target() {
    let state = gui_state_for_span_tests();
    let runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let point = waveform_rect(&runtime).center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
}

#[test]
fn full_app_scene_routes_primary_waveform_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let press = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.75, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(press)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        runtime.bridge().state().waveform.play_mark_ratio(),
        Some(0.25)
    );
    assert_eq!(
        runtime.bridge().state().waveform.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.75))
    );
}

#[test]
fn full_app_scene_routes_primary_waveform_click_to_play_mark() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "play marking should request a repaint immediately after press"
    );
    let _ = runtime.take_repaint_requested();
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "click-to-play should request a repaint immediately after release"
    );

    assert_ratio_near(runtime.bridge().state().waveform.play_mark_ratio(), 0.42);
    assert_eq!(runtime.bridge().state().waveform.play_selection(), None);
    assert!(runtime.bridge().state().waveform.is_playing());
}

#[test]
fn transaction_list_modal_blocks_waveform_interaction_behind_it() {
    let mut state = gui_state_for_span_tests();
    state.transaction_list_open = true;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);

    assert_ne!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_ne!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(runtime.bridge().state().waveform.play_mark_ratio(), None);
    assert_eq!(runtime.bridge().state().waveform.play_selection(), None);
}

#[test]
fn full_app_scene_routes_secondary_waveform_edit_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let press = Point::new(rect.min.x + rect.width() * 0.2, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.7, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(press)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        runtime.bridge().state().waveform.edit_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.7))
    );
}

#[test]
fn full_app_scene_routes_secondary_waveform_click_to_edit_mark() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.38, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "edit marking should request a repaint immediately after press"
    );
    let _ = runtime.take_repaint_requested();
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "edit mark release should request a repaint"
    );

    assert_ratio_near(runtime.bridge().state().waveform.edit_mark_ratio(), 0.38);
    assert_eq!(runtime.bridge().state().waveform.edit_selection(), None);
}

#[test]
fn app_bridge_scene_routes_primary_waveform_selection_drag() {
    let state = gui_state_for_span_tests();
    let messages = Rc::new(RefCell::new(Vec::new()));
    let captured_messages = Rc::clone(&messages);
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::view)
        .update_with(move |state, message, context| {
            captured_messages.borrow_mut().push(message.clone());
            state.apply_message(message, context);
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let rect = *runtime
        .layout()
        .rects
        .get(&crate::native_app::test_support::WAVEFORM_WIDGET_ID)
        .expect("app bridge should lay out waveform widget");
    let press = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.75, rect.center().y);

    assert_eq!(
        runtime.widget_at(press),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(press)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    let messages = messages.borrow();
    assert!(
        messages.iter().any(|message| matches!(
            message,
            crate::native_app::test_support::GuiMessage::Waveform(
                WaveformInteraction::BeginSelection {
                    kind: WaveformSelectionKind::Play,
                    ..
                }
            )
        )),
        "{messages:?}"
    );
    assert!(
        messages.iter().any(|message| matches!(
            message,
            crate::native_app::test_support::GuiMessage::Waveform(
                WaveformInteraction::FinishSelection { .. }
            )
        )),
        "{messages:?}"
    );
}

#[test]
fn app_bridge_scene_preserves_waveform_drag_during_playback_frame_refresh() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);
    let messages = Rc::new(RefCell::new(Vec::new()));
    let captured_messages = Rc::clone(&messages);
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::view)
        .update_with(move |state, message, context| {
            captured_messages.borrow_mut().push(message.clone());
            state.apply_message(message, context);
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let rect = *runtime
        .layout()
        .rects
        .get(&crate::native_app::test_support::WAVEFORM_WIDGET_ID)
        .expect("app bridge should lay out waveform widget");
    let press = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.75, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(press)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime
            .bridge_mut()
            .animation_activity()
            .needs_frame_message()
    );
    assert!(runtime.bridge_mut().queue_animation_frame());
    runtime.drain_runtime_messages();
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    let messages = messages.borrow();
    assert!(
        messages.iter().any(|message| matches!(
            message,
            crate::native_app::test_support::GuiMessage::Waveform(
                WaveformInteraction::FinishSelection { .. }
            )
        )),
        "{messages:?}"
    );
}

#[test]
fn default_folder_browser_loads_assets_root() {
    let browser = crate::native_app::test_support::FolderBrowserState::load_default();
    assert!(browser.root_path().ends_with("assets"));
    assert_eq!(browser.source_labels(), vec![String::from("Assets")]);
    assert!(
        browser
            .selected_files()
            .iter()
            .any(|file| file.name == "portal_SS_kick_001.wav")
    );
    assert!(
        browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "portal_SS_kick_001.wav")
    );
}

#[test]
fn sample_browser_toggles_between_disk_and_metadata_label_names() {
    let (mut state, _source_root, tagged_file) =
        native_app_state_with_temp_sample("tag-toggle.wav");
    state.metadata_tags_by_file.insert(
        tagged_file,
        vec![String::from("kick"), String::from("warm")],
    );
    let disk_frame = crate::native_app::test_support::sample_browser(&mut state, false)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));
    assert!(disk_frame.paint_plan.contains_text("Disk"));

    state.apply_message(
        crate::native_app::test_support::GuiMessage::ToggleSampleNameViewMode,
        &mut ui::UpdateContext::default(),
    );
    let label_frame = crate::native_app::test_support::sample_browser(&mut state, false)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));

    assert!(label_frame.paint_plan.contains_text("Label"));
}

#[test]
fn waveform_loading_visual_paints_full_height_gray_fill_without_chrome() {
    let frame = waveform_loading_visual("kick.wav", 0.25)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 172.0));

    let fill_rects = frame.paint_plan.fill_rects().collect::<Vec<_>>();

    assert!(fill_rects.iter().any(|fill| {
        (fill.rect.width() - 180.0).abs() < 0.01
            && (fill.rect.height() - 172.0).abs() < 0.01
            && fill.rect.min.x == 0.0
            && fill.rect.min.y == 0.0
            && fill.color.r == 174
            && fill.color.g == 178
            && fill.color.b == 181
    }));
    assert!(
        frame.paint_plan.stroke_rects().next().is_none()
            && frame.paint_plan.text_runs().next().is_none()
    );
}
