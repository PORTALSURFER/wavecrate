use crate::app::controller::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::*;
use std::path::{Path, PathBuf};

#[test]
fn restore_meaningful_ui_snapshot_recovers_browser_folder_and_waveform_context() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a/one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("a/two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b/three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("a")).unwrap();
    std::fs::create_dir_all(source.root.join("b")).unwrap();
    write_test_wav(&source.root.join("a/one.wav"), &[0.0, 0.1]);
    controller.refresh_folder_browser_for_tests();
    let folder_a = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("a"))
        .unwrap();

    controller.replace_folder_selection(folder_a);
    controller.focus_browser_row_only(1);
    controller.toggle_browser_row_selection(0);
    controller.ui.browser.selection.autoscroll = false;
    controller
        .load_waveform_for_selection(&source, Path::new("a/one.wav"))
        .unwrap();
    let waveform_selection = SelectionRange::new(0.2, 0.6);
    let edit_selection = SelectionRange::new(0.25, 0.55).with_fade_out(0.1, 0.0);
    controller.apply_selection(Some(waveform_selection));
    controller.set_edit_selection_range(edit_selection);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.7,
    };
    controller.ui.waveform.cursor = Some(0.42);
    controller.ui.waveform.loop_enabled = true;

    let snapshot = controller.capture_meaningful_ui_snapshot();

    controller.clear_folder_selection();
    controller.focus_browser_row_only(0);
    controller.clear_browser_selection();
    controller.selection_state.ctx.selected_source = None;
    controller.sample_view.wav.selected_wav = None;
    controller.sample_view.wav.loaded_wav = None;
    controller.sample_view.wav.loaded_audio = None;
    controller.set_ui_loaded_wav(None);
    controller.apply_selection(None);
    controller.apply_edit_selection(None);
    controller.ui.waveform.view = crate::app::state::WaveformView::default();
    controller.ui.waveform.cursor = None;
    controller.ui.waveform.loop_enabled = false;
    controller.ui.browser.selection.autoscroll = true;
    controller.restore_meaningful_ui_snapshot(&snapshot);

    assert_eq!(controller.selected_source_id(), Some(source.id));
    assert_eq!(controller.selected_folder_paths(), vec![PathBuf::from("a")]);
    assert_eq!(
        controller.browser_selected_paths_snapshot(),
        vec![PathBuf::from("a/two.wav"), PathBuf::from("a/one.wav")]
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("a/one.wav"))
    );
    assert!(!controller.ui.browser.selection.autoscroll);
    assert_eq!(
        controller.selection_state.range.range(),
        Some(waveform_selection)
    );
    assert_eq!(
        controller.selection_state.edit_range.range(),
        Some(edit_selection)
    );
    assert_eq!(controller.ui.waveform.selection, Some(waveform_selection));
    assert_eq!(controller.ui.waveform.edit_selection, Some(edit_selection));
    assert_eq!(
        controller.ui.waveform.view,
        crate::app::state::WaveformView {
            start: 0.1,
            end: 0.7,
        }
    );
    assert_eq!(controller.ui.waveform.cursor, Some(0.42));
    assert!(controller.ui.waveform.loop_enabled);
    let restored_audio_path = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.relative_path.clone())
        .or_else(|| {
            controller
                .runtime
                .jobs
                .pending_audio()
                .map(|pending| pending.relative_path)
        });
    assert_eq!(restored_audio_path, Some(PathBuf::from("a/one.wav")));
}
