use super::*;

#[test]
fn default_window_title_marks_versioned_alpha_build() {
    let expected = format!(
        "Wavecrate {} b{} - Alpha",
        env!("CARGO_PKG_VERSION"),
        env!("WAVECRATE_BUILD_NUMBER")
    );

    assert_eq!(
        crate::native_app::test_support::shell::default_window_title(),
        expected
    );
}

#[test]
fn audio_settings_popover_opens_as_centered_floating_window() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio.settings_error = None;
    let frame = crate::native_app::test_support::settings::audio_settings_popover(&state)
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
    state.ui.settings.ui.audio_settings_open = true;
    let frame = crate::native_app::test_support::state::view(&mut state)
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
            <= crate::native_app::test_support::settings::AUDIO_SETTINGS_POPUP_HEIGHT + 1.0),
        "{audio_panel_fills:?}"
    );
}

#[test]
fn audio_settings_window_does_not_block_waveform_selection_messages() {
    let mut state = gui_state_for_span_tests();
    state.ui.settings.ui.audio_settings_open = true;
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Waveform(
            WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.45,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Waveform(
            WaveformInteraction::UpdateSelection {
                visible_ratio: 0.65,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Waveform(
            WaveformInteraction::FinishSelection {
                visible_ratio: 0.65,
            },
        ),
        &mut context,
    );

    assert_eq!(state.waveform.current.play_mark_ratio(), Some(0.45));
    assert_eq!(
        state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.45, 0.65))
    );
}
