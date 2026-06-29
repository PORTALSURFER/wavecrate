use crate::native_app::test_support::state::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, NativeAppState, NativeAppStateFixture,
    WaveformInteraction, default_gui_shortcuts,
};
use radiant::prelude as ui;

#[test]
fn escape_shortcut_routes_to_stop_playback() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(GuiMessage::StopPlayback));
    assert!(resolution.handled);
}

#[test]
fn loop_shortcut_routes_to_loop_toggle() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::L));

    assert_eq!(resolution.action, Some(GuiMessage::ToggleLoopPlayback));
    assert!(resolution.handled);
}

#[test]
fn space_shortcut_routes_to_play_selected_sample() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Space));

    assert_eq!(resolution.action, Some(GuiMessage::PlaySelectedSample));
    assert!(resolution.handled);
}

#[test]
fn sticky_random_space_shortcut_routes_to_random_sample_range() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.sticky_random_sample_range_playback = true;
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Space));

    assert_eq!(resolution.action, Some(GuiMessage::PlayRandomSampleRange));
    assert!(resolution.handled);
}

#[test]
fn shift_space_shortcut_routes_to_current_play_start() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_shift(ui::KeyCode::Space));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::PlayFromCurrentPlayStart)
    );
    assert!(resolution.handled);
}

#[test]
fn right_arrow_shortcut_routes_to_current_play_start() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::ArrowRight));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::PlayFromCurrentPlayStart)
    );
    assert!(resolution.handled);
}

#[test]
fn option_space_shortcut_routes_to_random_sample_range() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_alt(ui::KeyCode::Space));

    assert_eq!(resolution.action, Some(GuiMessage::PlayRandomSampleRange));
    assert!(resolution.handled);
}

#[test]
fn control_space_shortcut_routes_to_random_sample_range() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_control(ui::KeyCode::Space));

    assert_eq!(resolution.action, Some(GuiMessage::PlayRandomSampleRange));
    assert!(resolution.handled);
}

#[test]
fn e_shortcut_routes_to_extract_playmarked_range_command() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::E));

    assert_eq!(resolution.action, Some(GuiMessage::ExtractPlaymarkedRange));
    assert!(resolution.handled);
}

#[test]
fn command_left_shortcut_routes_to_previous_playback_history() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::ArrowLeft));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::PlayPreviousPlaybackHistory)
    );
    assert!(resolution.handled);
}

#[test]
fn command_right_shortcut_routes_to_next_playback_history() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::ArrowRight));

    assert_eq!(resolution.action, Some(GuiMessage::PlayNextPlaybackHistory));
    assert!(resolution.handled);
}

#[test]
fn x_shortcut_routes_to_toggle_selected_sample_and_advance() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::X));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::ToggleSelectedSampleAndAdvance)
    );
    assert!(resolution.handled);
}

#[test]
fn z_shortcut_routes_to_zoom_waveform_to_play_selection() {
    let state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Z));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::Waveform(
            WaveformInteraction::ZoomToPlaySelection
        ))
    );
    assert!(resolution.handled);
}

#[test]
fn f_shortcut_routes_to_focus_selected_starmap_node() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::F));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::FocusSelectedStarmapNode)
    );
    assert!(resolution.handled);
}

#[test]
fn w_shortcut_routes_to_global_context_menu() {
    let state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::W));

    assert_eq!(resolution.action, Some(GuiMessage::OpenContextMenu));
    assert!(resolution.handled);
}

#[test]
fn x_shortcut_routes_to_waveform_zoom_out_when_waveform_is_zoomed_in() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.waveform.current.set_play_selection_range(0.25, 0.50);
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::ZoomToPlaySelection);
    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::X));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::Waveform(WaveformInteraction::ZoomFull))
    );
    assert!(resolution.handled);
}

#[test]
fn shift_x_shortcut_routes_to_silence_margin_zoom_out_when_waveform_is_loaded() {
    let state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_shift(ui::KeyCode::X));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::Waveform(WaveformInteraction::ZoomOut {
            expand_silence_margin: true,
        }))
    );
    assert!(resolution.handled);
}

#[test]
fn x_shortcut_routes_to_waveform_zoom_full_from_silence_margin() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::ZoomOut {
            expand_silence_margin: true,
        });
    assert!(!state.waveform.current.fully_zoomed_out());

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::X));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::Waveform(WaveformInteraction::ZoomFull))
    );
    assert!(resolution.handled);
}

#[test]
fn command_x_shortcut_routes_to_cut_selected_files() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::X));

    assert_eq!(resolution.action, Some(GuiMessage::CutSelectedFiles));
    assert!(resolution.handled);
}

#[test]
fn command_v_shortcut_routes_to_paste_cut_files() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::V));

    assert_eq!(resolution.action, Some(GuiMessage::PasteCutFiles));
    assert!(resolution.handled);
}

#[test]
fn x_shortcut_is_consumed_while_renaming() {
    let (mut state, _source_root) = state_with_renamable_temp_sample("x-rename.wav");
    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("begin rename should not fail");

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::X));

    assert_eq!(resolution.action, None);
    assert!(resolution.handled);
}

#[test]
fn e_shortcut_is_consumed_while_renaming() {
    let (mut state, _source_root) = state_with_renamable_temp_sample("e-rename.wav");
    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("begin rename should not fail");

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::E));

    assert_eq!(resolution.action, None);
    assert!(resolution.handled);
}

fn state_with_renamable_temp_sample(name: &str) -> (NativeAppState, tempfile::TempDir) {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join(name);
    std::fs::write(&sample_path, []).expect("sample file");
    let folder_browser =
        FolderBrowserState::from_sample_sources(&[wavecrate::sample_sources::SampleSource::new(
            source_root.path().to_path_buf(),
        )]);
    let mut state = NativeAppStateFixture::default()
        .with_folder_browser(folder_browser)
        .build();
    state
        .library
        .folder_browser
        .select_file(sample_path.display().to_string());
    (state, source_root)
}

#[test]
fn backspace_shortcut_routes_to_delete_selected_item() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Backspace));

    assert_eq!(resolution.action, Some(GuiMessage::DeleteSelectedItem));
    assert!(resolution.handled);
}

#[test]
fn escape_shortcut_exits_collection_focus_before_stopping_playback() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    let collection = wavecrate::sample_sources::SampleCollection::new(0).expect("collection");
    state
        .library
        .folder_browser
        .apply_message(FolderBrowserMessage::ActivateCollection(collection));

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::FolderBrowser(
            FolderBrowserMessage::ExitCollectionFocus
        ))
    );
    assert!(resolution.handled);
}
