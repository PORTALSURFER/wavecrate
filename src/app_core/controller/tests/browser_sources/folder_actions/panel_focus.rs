use super::*;

#[test]
fn focus_folder_panel_preserves_existing_folder_selection() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    let folder_path = PathBuf::from("drums");
    std::fs::create_dir_all(source_root.join(&folder_path)).unwrap();
    browser_test_write_wav(
        &source_root.join(folder_path.join("clip.wav")),
        &[0.1, -0.1],
    );
    controller.add_source_from_path(source_root).unwrap();
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![browser_test_sample_entry(
        "drums/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let row_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == folder_path)
        .expect("failed to locate folder row");
    controller.replace_folder_selection(row_index);
    let selected_before = controller
        .folder_selection_for_filter()
        .cloned()
        .unwrap_or_default();
    let focused_before = controller.ui.sources.folders.focused;
    controller.ui.focus.context = FocusContext::Waveform;

    controller.apply_ui_action(NativeUiAction::Shell(
        crate::app_core::actions::NativeShellAction::FocusFolderPanel,
    ));

    assert_eq!(
        controller
            .folder_selection_for_filter()
            .cloned()
            .unwrap_or_default(),
        selected_before
    );
    assert_eq!(controller.ui.sources.folders.focused, focused_before);
    assert_eq!(controller.ui.focus.context, FocusContext::SourceFolders);
}
