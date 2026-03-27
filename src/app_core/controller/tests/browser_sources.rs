use super::*;

#[test]
fn apply_native_browser_normalize_routes_to_hotkey_behavior() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.apply_native_ui_action(NativeUiAction::NormalizeFocusedBrowserSample);

    assert!(
        controller
            .ui
            .status
            .text
            .contains("Focus a sample to normalize it"),
        "status was {:?}",
        controller.ui.status.text
    );
}

#[test]
/// Native folder-row focus action should select the clicked folder for filtering.
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

    if let Err(err) = controller.add_source_from_path(source_root) {
        panic!("failed to add source from path: {err}");
    }
    controller.select_source_by_index(0);
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

    controller.apply_native_ui_action(NativeUiAction::FocusFolderRow { index: row_index });

    let selected = controller
        .folder_selection_for_filter()
        .cloned()
        .unwrap_or_default();
    assert_eq!(selected, [folder_path].into_iter().collect::<BTreeSet<_>>());
    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
}

#[test]
/// Native folder-row activation should keep selection behavior and toggle expansion for folders.
fn activate_folder_row_action_selects_and_toggles_expansion() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    let folder_path = PathBuf::from("drums");
    let nested_path = folder_path.join("kicks");
    std::fs::create_dir_all(source_root.join(&nested_path)).unwrap();
    controller.add_source_from_path(source_root).unwrap();
    controller.select_source_by_index(0);
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

    controller.apply_native_ui_action(NativeUiAction::ActivateFolderRow { index: row_index });

    let selected = controller
        .folder_selection_for_filter()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        selected,
        [folder_path.clone()].into_iter().collect::<BTreeSet<_>>()
    );
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
/// Native source-row reload action should route to the targeted source index.
fn reload_source_row_action_selects_target_source() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("failed to create tempdir: {err}"),
    };
    let source_a = dir.path().join("source-a");
    let source_b = dir.path().join("source-b");
    if let Err(err) = std::fs::create_dir_all(&source_a) {
        panic!("failed to create source-a fixture: {err}");
    }
    if let Err(err) = std::fs::create_dir_all(&source_b) {
        panic!("failed to create source-b fixture: {err}");
    }
    if let Err(err) = controller.add_source_from_path(source_a) {
        panic!("failed to add source-a fixture: {err}");
    }
    if let Err(err) = controller.add_source_from_path(source_b) {
        panic!("failed to add source-b fixture: {err}");
    }

    controller.select_source_by_index(0);
    controller.apply_native_ui_action(NativeUiAction::ReloadSourceRow { index: 1 });

    assert_eq!(controller.ui.sources.selected, Some(1));
}

#[test]
/// Native source-row remove action should delete the targeted source.
fn remove_source_row_action_removes_target_source() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("failed to create tempdir: {err}"),
    };
    let source_a = dir.path().join("source-a");
    let source_b = dir.path().join("source-b");
    if let Err(err) = std::fs::create_dir_all(&source_a) {
        panic!("failed to create source-a fixture: {err}");
    }
    if let Err(err) = std::fs::create_dir_all(&source_b) {
        panic!("failed to create source-b fixture: {err}");
    }
    if let Err(err) = controller.add_source_from_path(source_a.clone()) {
        panic!("failed to add source-a fixture: {err}");
    }
    if let Err(err) = controller.add_source_from_path(source_b.clone()) {
        panic!("failed to add source-b fixture: {err}");
    }

    controller.apply_native_ui_action(NativeUiAction::RemoveSourceRow { index: 0 });

    assert_eq!(controller.ui.sources.rows.len(), 1);
    assert_eq!(
        controller.ui.sources.rows[0].path,
        source_b.to_string_lossy()
    );
}

#[test]
/// Native source-row focus action should select the source and move section focus to the source list.
fn focus_source_row_action_selects_source_and_focuses_sources_list() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_a = dir.path().join("source-a");
    let source_b = dir.path().join("source-b");
    std::fs::create_dir_all(&source_a).unwrap();
    std::fs::create_dir_all(&source_b).unwrap();
    controller.add_source_from_path(source_a).unwrap();
    controller.add_source_from_path(source_b).unwrap();
    controller.ui.focus.context = FocusContext::Waveform;

    controller.apply_native_ui_action(NativeUiAction::FocusSourceRow { index: 1 });

    assert_eq!(controller.ui.sources.selected, Some(1));
    assert_eq!(controller.ui.focus.context, FocusContext::SourcesList);
}

#[test]
/// Native source-panel focus should preserve the currently selected source row.
fn focus_sources_panel_preserves_selected_source_row() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_a = dir.path().join("source-a");
    let source_b = dir.path().join("source-b");
    std::fs::create_dir_all(&source_a).unwrap();
    std::fs::create_dir_all(&source_b).unwrap();
    controller.add_source_from_path(source_a).unwrap();
    controller.add_source_from_path(source_b).unwrap();
    controller.select_source_by_index(1);
    controller.ui.focus.context = FocusContext::Waveform;

    controller.apply_native_ui_action(NativeUiAction::FocusSourcesPanel);

    assert_eq!(controller.ui.sources.selected, Some(1));
    assert_eq!(controller.ui.focus.context, FocusContext::SourcesList);
}

#[test]
/// Native folder-panel focus should not disturb the existing folder selection state.
fn focus_folder_panel_preserves_existing_folder_selection() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_root = dir.path().join("source");
    let folder_path = PathBuf::from("drums");
    std::fs::create_dir_all(source_root.join(&folder_path)).unwrap();
    controller.add_source_from_path(source_root).unwrap();
    controller.select_source_by_index(0);
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

    controller.apply_native_ui_action(NativeUiAction::FocusFolderPanel);

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
/// Loading configuration should prune transient benchmark-only sources.
fn apply_configuration_prunes_transient_benchmark_sources() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let retained_root = match tempdir() {
        Ok(dir) => {
            let root = dir.path().join("user-source");
            if let Err(err) = std::fs::create_dir_all(&root) {
                panic!("failed to create retained fixture: {err}");
            }
            std::mem::forget(dir);
            root
        }
        Err(err) => panic!("failed to create retained tempdir: {err}"),
    };
    let transient_root = std::env::temp_dir()
        .join("sempal-test-gui-source")
        .join("gui-source");
    if let Err(err) = std::fs::create_dir_all(&transient_root) {
        panic!("failed to create transient fixture: {err}");
    }
    let cfg = crate::sample_sources::config::AppConfig {
        sources: vec![
            crate::sample_sources::SampleSource::new(transient_root),
            crate::sample_sources::SampleSource::new(retained_root.clone()),
        ],
        ..crate::sample_sources::config::AppConfig::default()
    };

    if let Err(err) = controller.apply_configuration(cfg) {
        panic!("failed to apply configuration: {err}");
    }

    assert_eq!(controller.ui.sources.rows.len(), 1);
    assert_eq!(
        controller.ui.sources.rows[0].path,
        retained_root.to_string_lossy()
    );
}
