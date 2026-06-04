use super::super::waveform_panel::waveform_loading_visual;
use super::*;

#[test]
fn default_window_title_marks_alpha_build() {
    assert_eq!(
        super::super::launch::DEFAULT_WINDOW_TITLE,
        "Wavecrate - alpha"
    );
}

#[test]
fn audio_settings_popover_opens_as_centered_floating_window() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_error = None;
    let frame = super::super::audio_settings_popover(&state)
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
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_open = true;
    let frame = super::super::view(&mut state)
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
        audio_panel_fills
            .iter()
            .all(|rect| rect.height() <= super::super::AUDIO_SETTINGS_POPUP_HEIGHT + 1.0),
        "{audio_panel_fills:?}"
    );
}

#[test]
fn audio_settings_window_does_not_block_waveform_selection_messages() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_open = true;
    let mut context = ui::UpdateContext::default();

    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.45,
        }),
        &mut context,
    );
    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.65,
        }),
        &mut context,
    );
    state.apply_message(
        super::super::GuiMessage::Waveform(WaveformInteraction::FinishSelection {
            visible_ratio: 0.65,
        }),
        &mut context,
    );

    assert_eq!(state.waveform.play_mark_ratio(), Some(0.45));
    assert_eq!(
        state.waveform.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.45, 0.65))
    );
}

#[test]
fn default_folder_browser_loads_assets_root() {
    let browser = super::super::FolderBrowserState::load_default();
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
    let (mut state, _source_root, tagged_file) = gui_state_with_temp_sample("tag-toggle.wav");
    state.metadata_tags_by_file.insert(
        tagged_file,
        vec![String::from("kick"), String::from("warm")],
    );
    let disk_frame = super::super::sample_browser(&mut state, false)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));
    assert!(disk_frame.paint_plan.contains_text("Disk"));

    state.apply_message(
        super::super::GuiMessage::ToggleSampleNameViewMode,
        &mut ui::UpdateContext::default(),
    );
    let label_frame = super::super::sample_browser(&mut state, false)
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
