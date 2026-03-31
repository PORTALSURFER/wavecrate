use super::*;

#[test]
fn apply_native_toggle_browser_sample_mark_marks_focused_browser_row() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().expect("temp source root");
    let source_root = dir.path().join("source");
    std::fs::create_dir_all(&source_root).expect("create source root");
    controller
        .add_source_from_path(source_root)
        .expect("source should be added");
    controller.select_source_by_index(0);
    let source = controller
        .current_source()
        .expect("current source should exist");
    controller.settings.feature_flags.autoplay_selection = false;
    controller.set_wav_entries_for_tests(vec![
        WavEntry {
            relative_path: PathBuf::from("marked.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-a")),
            tag: Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        },
        WavEntry {
            relative_path: PathBuf::from("next.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-b")),
            tag: Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        },
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller.apply_native_ui_action(NativeUiAction::ToggleBrowserSampleMark);

    assert!(controller.browser_sample_marked(&source.id, Path::new("marked.wav")));
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("next.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("next.wav"))
    );
    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
}
