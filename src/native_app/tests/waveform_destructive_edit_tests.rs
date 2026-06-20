use super::*;
use crate::native_app::test_support::state::GuiMessage;

#[test]
fn crop_shortcut_routes_to_waveform_crop_request() {
    let state = crate::native_app::test_support::state::NativeAppStateFixture::default().build();
    let resolution = crate::native_app::test_support::state::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::C));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::RequestCropWaveformSelection)
    );
    assert!(resolution.handled);
}

#[test]
fn trim_shortcut_routes_to_waveform_trim_request() {
    let state = crate::native_app::test_support::state::NativeAppStateFixture::default().build();
    let resolution = crate::native_app::test_support::state::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::D));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::RequestTrimWaveformSelection)
    );
    assert!(resolution.handled);
}

#[test]
fn command_extract_shortcut_routes_to_extract_and_trim_request() {
    let state = crate::native_app::test_support::state::NativeAppStateFixture::default().build();
    let resolution = crate::native_app::test_support::state::default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::with_command(ui::KeyCode::E));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::RequestExtractAndTrimWaveformSelection)
    );
    assert!(resolution.handled);
}

#[test]
fn crop_request_uses_play_selection_when_no_edit_selection_exists() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("crop.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("load waveform");
    state.ui.settings.persisted.controls.destructive_yolo_mode = false;

    select_waveform_range(&mut state, WaveformSelectionKind::Play, 0.25, 0.5);

    state.apply_message(
        GuiMessage::RequestCropWaveformSelection,
        &mut ui::UiUpdateContext::default(),
    );

    let pending = state
        .ui
        .browser_interaction
        .pending_waveform_destructive_edit
        .as_ref()
        .expect("crop request should prompt");
    assert_eq!(
        pending.prompt.edit,
        crate::native_app::app::WaveformDestructiveEditKind::CropSelection
    );
    assert!((pending.selection.start() - 0.25).abs() < 0.001);
    assert!((pending.selection.end() - 0.5).abs() < 0.001);
}

#[test]
fn trim_request_uses_play_selection_when_no_edit_selection_exists() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("trim.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("load waveform");
    state.ui.settings.persisted.controls.destructive_yolo_mode = false;

    select_waveform_range(&mut state, WaveformSelectionKind::Play, 0.25, 0.5);

    state.apply_message(
        GuiMessage::RequestTrimWaveformSelection,
        &mut ui::UiUpdateContext::default(),
    );

    let pending = state
        .ui
        .browser_interaction
        .pending_waveform_destructive_edit
        .as_ref()
        .expect("trim request should prompt");
    assert_eq!(
        pending.prompt.edit,
        crate::native_app::app::WaveformDestructiveEditKind::TrimSelection
    );
    assert!((pending.selection.start() - 0.25).abs() < 0.001);
    assert!((pending.selection.end() - 0.5).abs() < 0.001);
}

#[test]
fn extract_and_trim_request_uses_play_selection_when_no_edit_selection_exists() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("extract-trim.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("load waveform");
    state.ui.settings.persisted.controls.destructive_yolo_mode = false;

    select_waveform_range(&mut state, WaveformSelectionKind::Play, 0.25, 0.5);

    state.apply_message(
        GuiMessage::RequestExtractAndTrimWaveformSelection,
        &mut ui::UiUpdateContext::default(),
    );

    let pending = state
        .ui
        .browser_interaction
        .pending_waveform_destructive_edit
        .as_ref()
        .expect("extract-and-trim request should prompt");
    assert_eq!(
        pending.prompt.edit,
        crate::native_app::app::WaveformDestructiveEditKind::ExtractAndTrimSelection
    );
    assert!(
        pending
            .prompt
            .message
            .contains("extract the selected region")
    );
    assert!((pending.selection.start() - 0.25).abs() < 0.001);
    assert!((pending.selection.end() - 0.5).abs() < 0.001);
}

#[test]
fn crop_request_rewrites_file_and_undo_restores_original_audio() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("crop.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("load waveform");
    state.ui.settings.persisted.controls.destructive_yolo_mode = true;

    select_waveform_range(&mut state, WaveformSelectionKind::Play, 0.25, 0.5);

    state.apply_message(
        GuiMessage::RequestCropWaveformSelection,
        &mut ui::UiUpdateContext::default(),
    );

    assert_samples_close(&read_test_wav_f32(&path), &[2_000.0, 3_000.0]);
    assert!(state.ui.status.sample.contains("Cropped"));

    state.apply_message(
        GuiMessage::UndoTransaction,
        &mut ui::UiUpdateContext::default(),
    );

    assert_samples_close(
        &read_test_wav_f32(&path),
        &[
            0.0, 1_000.0, 2_000.0, 3_000.0, 4_000.0, 5_000.0, 6_000.0, 7_000.0,
        ],
    );
}

#[test]
fn trim_request_rewrites_file_and_undo_restores_original_audio() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("trim.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("load waveform");
    state.ui.settings.persisted.controls.destructive_yolo_mode = true;

    select_waveform_range(&mut state, WaveformSelectionKind::Play, 0.25, 0.5);

    state.apply_message(
        GuiMessage::RequestTrimWaveformSelection,
        &mut ui::UiUpdateContext::default(),
    );

    assert_samples_close(
        &read_test_wav_f32(&path),
        &[0.0, 1_000.0, 4_000.0, 5_000.0, 6_000.0, 7_000.0],
    );
    assert!(state.ui.status.sample.contains("Trimmed"));

    state.apply_message(
        GuiMessage::UndoTransaction,
        &mut ui::UiUpdateContext::default(),
    );

    assert_samples_close(
        &read_test_wav_f32(&path),
        &[
            0.0, 1_000.0, 2_000.0, 3_000.0, 4_000.0, 5_000.0, 6_000.0, 7_000.0,
        ],
    );
}

#[test]
fn extract_and_trim_request_extracts_selection_trims_source_and_undo_redo_roundtrips() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("extract-trim.wav");
    let path = PathBuf::from(&selected_file);
    let extracted = path.with_file_name("extract-trim_extraction.wav");
    write_test_wav_i16(&path, &[0, 1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000]);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("load waveform");
    state.ui.settings.persisted.controls.destructive_yolo_mode = true;

    select_waveform_range(&mut state, WaveformSelectionKind::Play, 0.25, 0.5);

    state.apply_message(
        GuiMessage::RequestExtractAndTrimWaveformSelection,
        &mut ui::UiUpdateContext::default(),
    );

    assert_samples_close(&read_test_wav_f32(&extracted), &[2_000.0, 3_000.0]);
    assert_samples_close(
        &read_test_wav_f32(&path),
        &[0.0, 1_000.0, 4_000.0, 5_000.0, 6_000.0, 7_000.0],
    );
    assert!(state.ui.status.sample.contains("Extracted and trimmed"));

    state.apply_message(
        GuiMessage::UndoTransaction,
        &mut ui::UiUpdateContext::default(),
    );

    assert!(
        !extracted.exists(),
        "undo should remove the generated extraction file"
    );
    assert_samples_close(
        &read_test_wav_f32(&path),
        &[
            0.0, 1_000.0, 2_000.0, 3_000.0, 4_000.0, 5_000.0, 6_000.0, 7_000.0,
        ],
    );

    state.apply_message(
        GuiMessage::RedoTransaction,
        &mut ui::UiUpdateContext::default(),
    );

    assert_samples_close(&read_test_wav_f32(&extracted), &[2_000.0, 3_000.0]);
    assert_samples_close(
        &read_test_wav_f32(&path),
        &[0.0, 1_000.0, 4_000.0, 5_000.0, 6_000.0, 7_000.0],
    );
}

fn select_waveform_range(
    state: &mut crate::native_app::test_support::state::NativeAppState,
    kind: WaveformSelectionKind,
    start: f32,
    end: f32,
) {
    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::BeginSelection {
            kind,
            visible_ratio: start,
        }),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::UpdateSelection { visible_ratio: end }),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::FinishSelection { visible_ratio: end }),
        &mut ui::UiUpdateContext::default(),
    );
}

fn assert_samples_close(actual: &[f32], expected_i16: &[f32]) {
    assert_eq!(actual.len(), expected_i16.len(), "sample length mismatch");
    for (actual, expected) in actual.iter().zip(expected_i16.iter()) {
        let expected = *expected / 32_768.0;
        assert!(
            (*actual - expected).abs() < 0.000_1,
            "expected {expected}, got {actual}"
        );
    }
}
