use super::*;

#[test]
fn activate_folder_row_action_selects_and_toggles_expansion() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    let folder_path = PathBuf::from("drums");
    let nested_path = folder_path.join("kicks");
    std::fs::create_dir_all(source_root.join(&nested_path)).unwrap();
    browser_test_write_wav(
        &source_root.join(nested_path.join("tight.wav")),
        &[0.1, -0.1],
    );
    controller.add_source_from_path(source_root).unwrap();
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![browser_test_sample_entry(
        "drums/kicks/tight.wav",
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
        .expect("failed to locate folder row index");
    controller.toggle_folder_expanded(row_index);
    let row_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == folder_path)
        .expect("failed to relocate folder row index");

    controller.apply_ui_action(NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow {
            index: row_index,
        },
    ));

    let selected = controller
        .folder_selection_for_filter()
        .cloned()
        .unwrap_or_default();
    assert_eq!(selected, [folder_path].into_iter().collect::<BTreeSet<_>>());
    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == nested_path)
    );
}

#[test]
fn toggle_folder_row_expanded_action_toggles_expansion_immediately() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    let folder_path = PathBuf::from("drums");
    let nested_path = folder_path.join("kicks");
    std::fs::create_dir_all(source_root.join(&nested_path)).unwrap();
    browser_test_write_wav(
        &source_root.join(nested_path.join("tight.wav")),
        &[0.1, -0.1],
    );
    controller.add_source_from_path(source_root).unwrap();
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![browser_test_sample_entry(
        "drums/kicks/tight.wav",
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
        .expect("failed to locate folder row index");
    controller.toggle_folder_expanded(row_index);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != nested_path)
    );

    let row_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == folder_path)
        .expect("failed to relocate collapsed folder row index");
    controller.apply_ui_action(NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::ToggleFolderRowExpanded {
            index: row_index,
        },
    ));

    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == nested_path)
    );
}
