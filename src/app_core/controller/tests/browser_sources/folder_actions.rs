use super::*;

#[test]
fn toggle_focused_folder_selection_action_preserves_focus_and_anchor() {
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

    controller.apply_native_ui_action(NativeUiAction::ToggleFocusedFolderSelection);

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

    controller.apply_native_ui_action(NativeUiAction::ToggleFocusedFolderSelection);

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
    if let Some(parent) = sample_path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            panic!("failed to create sample fixture directory: {err}");
        }
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

    controller.apply_native_ui_action(NativeUiAction::FocusFolderRow {
        pane: None,
        index: row_index,
    });

    let selected = controller
        .folder_selection_for_filter()
        .cloned()
        .unwrap_or_default();
    assert_eq!(selected, [folder_path].into_iter().collect::<BTreeSet<_>>());
    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
}

#[test]
fn activate_folder_row_action_selects_and_toggles_expansion() {
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

    controller.apply_native_ui_action(NativeUiAction::ActivateFolderRow {
        pane: None,
        index: row_index,
    });

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
fn focus_folder_panel_preserves_existing_folder_selection() {
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

    controller.apply_native_ui_action(NativeUiAction::FocusFolderPanel { pane: None });

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

#[test]
fn toggle_show_all_folders_action_updates_folder_tree_mode() {
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
            .all(|row| row.path != PathBuf::from("drums/empty"))
    );

    controller.apply_native_ui_action(NativeUiAction::ToggleShowAllFolders { pane: None });

    assert!(controller.ui.sources.folders.show_all_folders);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/empty"))
    );
}

#[test]
fn toggle_folder_flattened_view_action_updates_folder_scope_mode() {
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
        .position(|row| row.path == PathBuf::from("drums"))
        .expect("failed to locate folder row");
    controller.replace_folder_selection(row_index);

    assert_eq!(
        browser_visible_paths(&mut controller),
        vec![PathBuf::from("drums/root.wav")]
    );
    assert!(!controller.ui.sources.folders.flattened_view);

    controller.apply_native_ui_action(NativeUiAction::ToggleFolderFlattenedView { pane: None });

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

    controller.apply_native_ui_action(NativeUiAction::ActivateFolderRow {
        pane: None,
        index: 0,
    });
    assert!(!controller.ui.sources.folders.flattened_view);
    assert_eq!(
        browser_visible_paths(&mut controller),
        vec![PathBuf::from("root.wav")]
    );

    controller.apply_native_ui_action(NativeUiAction::ActivateFolderRow {
        pane: None,
        index: 0,
    });
    assert!(!controller.ui.sources.folders.flattened_view);
    assert_eq!(
        browser_visible_paths(&mut controller),
        vec![PathBuf::from("root.wav")]
    );
}
