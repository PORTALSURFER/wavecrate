use super::*;
use std::path::Path;

#[test]
fn toggle_show_all_folders_action_updates_folder_tree_mode() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    std::fs::create_dir_all(source_root.join("drums/empty")).unwrap();
    std::fs::create_dir_all(source_root.join("drums/kicks")).unwrap();
    controller
        .add_source_from_path(source_root.clone())
        .unwrap();
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![browser_test_sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    assert!(!controller.ui.sources.folders.show_all_folders);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path.as_path() != Path::new("drums/empty"))
    );

    controller.apply_ui_action(NativeUiAction::Shell(
        crate::app_core::actions::NativeShellAction::ToggleShowAllFolders,
    ));

    assert!(controller.ui.sources.folders.show_all_folders);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path.as_path() == Path::new("drums/empty"))
    );
}

#[test]
fn toggle_folder_flattened_view_action_updates_folder_scope_mode() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    std::fs::create_dir_all(source_root.join("drums/kicks")).unwrap();
    controller
        .add_source_from_path(source_root.clone())
        .unwrap();
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![
        browser_test_sample_entry("drums/root.wav", crate::sample_sources::Rating::NEUTRAL),
        browser_test_sample_entry(
            "drums/kicks/tight.wav",
            crate::sample_sources::Rating::NEUTRAL,
        ),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let row_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path.as_path() == Path::new("drums"))
        .expect("failed to locate folder row");
    controller.replace_folder_selection(row_index);

    assert_eq!(
        browser_visible_paths(&mut controller),
        vec![PathBuf::from("drums/root.wav")]
    );
    assert!(!controller.ui.sources.folders.flattened_view);

    controller.apply_ui_action(NativeUiAction::Shell(
        crate::app_core::actions::NativeShellAction::ToggleFolderFlattenedView,
    ));

    assert!(controller.ui.sources.folders.flattened_view);
    assert_eq!(
        browser_visible_paths(&mut controller),
        vec![
            PathBuf::from("drums/root.wav"),
            PathBuf::from("drums/kicks/tight.wav"),
        ]
    );
}

#[test]
fn activate_root_folder_row_action_keeps_flattened_view_stable() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    std::fs::create_dir_all(source_root.join("drums")).unwrap();
    controller.add_source_from_path(source_root).unwrap();
    controller.select_source_by_index(0);
    controller.set_wav_entries_for_tests(vec![
        browser_test_sample_entry("root.wav", crate::sample_sources::Rating::NEUTRAL),
        browser_test_sample_entry("drums/clip.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.apply_ui_action(NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { index: 0 },
    ));
    assert!(!controller.ui.sources.folders.flattened_view);
    assert_eq!(
        browser_visible_paths(&mut controller),
        vec![PathBuf::from("root.wav")]
    );

    controller.apply_ui_action(NativeUiAction::SourcesAndFolders(
        crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { index: 0 },
    ));
    assert!(!controller.ui.sources.folders.flattened_view);
    assert_eq!(
        browser_visible_paths(&mut controller),
        vec![PathBuf::from("root.wav")]
    );
}
