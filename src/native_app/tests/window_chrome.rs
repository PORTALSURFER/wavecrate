use super::*;
use crate::native_app::app_chrome::waveform_panel::waveform_loading_visual;
use radiant::runtime::{NativeFileDrop, RuntimeBridge, SurfaceRuntime};
use std::{cell::RefCell, rc::Rc};
use winit::{dpi::PhysicalPosition, event::MouseButton};

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

struct NativePointerShellHarness {
    runtime: NativeRuntimeForTests,
    last_cursor: Option<Point>,
    dpi_scale: radiant::theme::DpiScale,
}

impl NativePointerShellHarness {
    fn new(state: NativeAppState) -> Self {
        Self {
            runtime: native_runtime_for_tests(state, Vector2::new(900.0, 620.0)),
            last_cursor: None,
            dpi_scale: radiant::theme::DpiScale::ONE,
        }
    }

    fn runtime(&self) -> &NativeRuntimeForTests {
        &self.runtime
    }

    fn cursor_moved_logical(&mut self, point: Point) -> Option<u64> {
        self.last_cursor = Some(point);
        self.runtime.dispatch_event(Event::pointer_move(point))
    }

    fn cursor_moved_physical(&mut self, position: PhysicalPosition<f64>) -> Option<u64> {
        let point = Point::new(
            self.dpi_scale.physical_to_logical(position.x as f32),
            self.dpi_scale.physical_to_logical(position.y as f32),
        );
        self.cursor_moved_logical(point)
    }

    fn mouse_pressed(&mut self, button: MouseButton) -> Option<u64> {
        let position = self.last_cursor?;
        self.runtime.dispatch_event(Event::pointer_press(
            position,
            pointer_button(button)?,
            Default::default(),
        ))
    }

    fn mouse_released(&mut self, button: MouseButton) -> Option<u64> {
        let position = self.last_cursor?;
        self.runtime.dispatch_event(Event::pointer_release(
            position,
            pointer_button(button)?,
            Default::default(),
        ))
    }
}

fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Auxiliary,
        _ => return None,
    })
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
    state.audio.settings_error = None;
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
    state.settings_ui.audio_settings_open = true;
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
    state.settings_ui.audio_settings_open = true;
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
fn stale_waveform_loading_label_does_not_mask_waveform_hit_target() {
    let mut state = gui_state_for_span_tests();
    state.waveform_load.label = Some(String::from("previous.wav"));
    state.waveform_load.progress = 0.5;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_ratio_near(runtime.bridge().state().waveform.play_mark_ratio(), 0.42);
}

#[test]
fn stale_waveform_drop_hover_does_not_mask_waveform_hit_target() {
    let mut state = gui_state_for_span_tests();
    state.browser_interaction.native_file_drop_hover =
        Some(crate::native_app::test_support::NativeFileDropHover {
            path: PathBuf::from("stale.wav"),
            supported: true,
        });
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.38, rect.center().y);

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_ratio_near(runtime.bridge().state().waveform.edit_mark_ratio(), 0.38);
}

#[test]
fn active_waveform_sample_load_masks_waveform_hit_target() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("blocking-load.wav");
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: selected_file,
            modifiers: Default::default(),
        },
        &mut ui::UpdateContext::default(),
    );
    assert!(state.waveform_input_blocked_by_sample_load());
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_ne!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(runtime.bridge().state().waveform.play_mark_ratio(), None);
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
    assert!(
        runtime
            .bridge()
            .state()
            .background
            .audio_open_task
            .active()
            .is_some(),
        "waveform click playback should queue audio output immediately"
    );
    assert!(
        runtime
            .bridge()
            .state()
            .audio
            .pending_playback_start
            .is_some(),
        "waveform click should keep playback pending until audio output opens"
    );
    assert!(
        !runtime
            .bridge()
            .state()
            .sample_status
            .contains("Playback unavailable"),
        "waveform click should not present pending audio output as a playback failure"
    );
}

#[test]
fn full_app_scene_primary_waveform_click_starts_audio_playback() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    let state = runtime.bridge().state();
    assert_ratio_near(state.waveform.play_mark_ratio(), 0.42);
    assert!(state.waveform.is_playing());
    let (start, end) = state
        .audio
        .current_playback_span
        .expect("waveform click should set playback span");
    assert!((start - 0.42).abs() <= 0.000_001, "start was {start}");
    assert_eq!(end, 1.0);
    assert!(
        state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.progress().is_some()),
        "primary waveform click should start the audio player"
    );
}

#[test]
fn native_pointer_shell_routes_primary_waveform_click_to_play_mark() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        harness.cursor_moved_physical(PhysicalPosition::new(point.x as f64, point.y as f64)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_ratio_near(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .play_mark_ratio(),
        0.42,
    );
    assert_eq!(
        harness.runtime().bridge().state().waveform.play_selection(),
        None
    );
    assert!(harness.runtime().bridge().state().waveform.is_playing());
}

#[test]
fn native_pointer_shell_routes_primary_waveform_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let press = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.75, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(press),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.cursor_moved_logical(drag),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .play_mark_ratio(),
        Some(0.25)
    );
    assert_eq!(
        harness.runtime().bridge().state().waveform.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.75))
    );
}

#[test]
fn transaction_list_modal_blocks_waveform_interaction_behind_it() {
    let mut state = gui_state_for_span_tests();
    state.chrome.transaction_list_open = true;
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
fn native_pointer_shell_routes_secondary_waveform_click_to_edit_mark() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let point = Point::new(rect.min.x + rect.width() * 0.38, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(point),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Right),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Right),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_ratio_near(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .edit_mark_ratio(),
        0.38,
    );
    assert_eq!(
        harness.runtime().bridge().state().waveform.edit_selection(),
        None
    );
}

#[test]
fn native_pointer_shell_routes_secondary_waveform_edit_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let press = Point::new(rect.min.x + rect.width() * 0.2, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.7, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(press),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Right),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.cursor_moved_logical(drag),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Right),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        harness.runtime().bridge().state().waveform.edit_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.7))
    );
}

#[test]
fn native_pointer_shell_preserves_waveform_drag_after_playback_frame_refresh() {
    let mut state = gui_state_for_span_tests();
    state.waveform.start_playback(0.25);
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let press = Point::new(rect.min.x + rect.width() * 0.3, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.8, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(press),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    harness.runtime.bridge_mut().queue_animation_frame();
    harness.runtime.drain_runtime_messages();
    assert_eq!(
        harness.cursor_moved_logical(drag),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        harness.runtime().bridge().state().waveform.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.3, 0.8))
    );
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
fn app_bridge_scene_routes_native_file_drop_to_waveform_view() {
    let root = temp_gui_root("wavecrate-app-bridge-native-drop-root");
    let external_root = temp_gui_root("wavecrate-app-bridge-native-drop-external");
    let loops = root.join("loops");
    fs::create_dir_all(&loops).expect("create loops");
    let source = external_root.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.folder_browser.apply_message(
        crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
        ),
    );
    let messages = Rc::new(RefCell::new(Vec::new()));
    let captured_messages = Rc::clone(&messages);
    let waveform_loading_label = Rc::new(RefCell::new(None));
    let captured_waveform_loading_label = Rc::clone(&waveform_loading_label);
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::view)
        .reducer(move |state, message, context| {
            captured_messages.borrow_mut().push(message.clone());
            state.apply_message(message, context);
            *captured_waveform_loading_label.borrow_mut() = state.waveform_load.label.clone();
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let rect = *runtime
        .layout()
        .rects
        .get(&crate::native_app::test_support::WAVEFORM_WIDGET_ID)
        .expect("app bridge should lay out waveform widget");

    runtime.dispatch_native_file_drop(NativeFileDrop::dropped(source, Some(rect.center()), None));

    let copied = loops.join("kick.wav");
    assert!(copied.is_file());
    assert_eq!(waveform_loading_label.borrow().as_deref(), Some("kick.wav"));
    let messages = messages.borrow();
    assert!(
        messages.iter().any(|message| matches!(
            message,
            crate::native_app::test_support::GuiMessage::WaveformFileDrop(_)
        )),
        "{messages:?}"
    );
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(external_root);
}

#[test]
fn app_bridge_scene_routes_targetless_native_file_drop_to_single_waveform_target() {
    let root = temp_gui_root("wavecrate-app-bridge-targetless-native-drop-root");
    let external_root = temp_gui_root("wavecrate-app-bridge-targetless-native-drop-external");
    let loops = root.join("loops");
    fs::create_dir_all(&loops).expect("create loops");
    let source = external_root.join("kick.wav");
    write_test_wav_i16(&source, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.folder_browser.apply_message(
        crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
            loops.display().to_string(),
        ),
    );
    let waveform_loading_label = Rc::new(RefCell::new(None));
    let captured_waveform_loading_label = Rc::clone(&waveform_loading_label);
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::view)
        .reducer(move |state, message, context| {
            state.apply_message(message, context);
            *captured_waveform_loading_label.borrow_mut() = state.waveform_load.label.clone();
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));

    runtime.dispatch_native_file_drop(NativeFileDrop::dropped(source, None, None));

    let copied = loops.join("kick.wav");
    assert!(copied.is_file());
    assert_eq!(waveform_loading_label.borrow().as_deref(), Some("kick.wav"));
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(external_root);
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
    state.metadata.tags_by_file.insert(
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
