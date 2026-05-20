use super::*;
use crate::app::controller::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
};
use crate::app::controller::ui::hotkeys::HotkeyCommand;
use crate::sample_sources::Rating;
use crate::selection::SelectionRange;
use std::path::PathBuf;

#[test]
fn test_delete_loaded_sample_navigation() {
    let temp = tempfile::tempdir().unwrap();
    let trash_root = temp.path().join("trash");
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.settings.trash_folder = Some(trash_root.clone());
    controller.ui.trash_folder = Some(trash_root.clone());

    load_waveform_selection(
        &mut controller,
        &source,
        "one.wav",
        &[0.1, -0.1],
        SelectionRange::new(0.0, 1.0),
    );

    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .unwrap()
            .relative_path,
        PathBuf::from("one.wav")
    );

    let result = handle_waveform_command(
        &mut controller.hotkeys_ctrl(),
        HotkeyCommand::DeleteLoadedSample,
    );
    assert!(result);
    assert!(trash_root.join("one.wav").exists());
}
