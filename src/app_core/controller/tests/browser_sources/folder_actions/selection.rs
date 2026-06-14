use super::*;

#[test]
fn toggle_focused_folder_selection_action_preserves_focus_and_anchor() {
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
    controller.focus_folder_row(row_index);

    controller.apply_ui_action(NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection,
    ));

    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
    assert_eq!(
        controller
            .folder_selection_for_filter()
            .cloned()
            .unwrap_or_default(),
        [folder_path.clone()].into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(
        controller
            .current_folder_model()
            .and_then(|model| model.selection_anchor.clone()),
        Some(folder_path.clone())
    );

    controller.apply_ui_action(NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection,
    ));

    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
    assert!(
        controller
            .folder_selection_for_filter()
            .cloned()
            .unwrap_or_default()
            .is_empty()
    );
    assert!(
        controller
            .current_folder_model()
            .is_some_and(|model| model.selection_anchor.is_none())
    );
}

#[test]
fn focus_folder_row_action_replaces_folder_selection() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("failed to create tempdir: {err}"),
    };
    let source_root = dir.path().join("source");
    let folder_path = PathBuf::from("drums");
    if let Err(err) = std::fs::create_dir_all(source_root.join(&folder_path)) {
        panic!("failed to create folder fixture: {err}");
    }
    let sample_path = source_root.join(folder_path.join("clip.wav"));
    if let Some(parent) = sample_path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        panic!("failed to create sample fixture directory: {err}");
    }
    browser_test_write_wav(&sample_path, &[0.1, -0.1]);

    if let Err(err) = controller.add_source_from_path(source_root) {
        panic!("failed to add source from path: {err}");
    }
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![browser_test_sample_entry(
        "drums/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let row_index = match controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == folder_path)
    {
        Some(index) => index,
        None => panic!("failed to locate folder row index"),
    };

    controller.apply_ui_action(NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderRow { index: row_index },
    ));

    let selected = controller
        .folder_selection_for_filter()
        .cloned()
        .unwrap_or_default();
    assert_eq!(selected, [folder_path].into_iter().collect::<BTreeSet<_>>());
    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
}
