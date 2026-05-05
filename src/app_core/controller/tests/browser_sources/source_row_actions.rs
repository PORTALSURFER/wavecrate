use super::*;
use crate::app::state::FolderPaneId;

#[test]
/// Reloading by source-row action uses the visible single active source list.
fn reload_source_row_action_uses_single_active_source_list() {
    let _sandbox = ControllerPersistenceSandbox::new();
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
    let source_b_id = controller
        .source_id_for_index(1)
        .expect("source-b id should exist");
    controller.apply_native_ui_action(NativeUiAction::ReloadSourceRow { index: 1 });

    assert_eq!(
        controller.folder_pane_source(FolderPaneId::Upper),
        Some(source_b_id)
    );
    assert_eq!(controller.active_folder_pane(), FolderPaneId::Upper);
    assert_eq!(controller.ui.sources.selected, Some(1));
}

#[test]
/// Removing a source row acts on the visible list without switching panes.
fn remove_source_row_action_removes_clicked_source_from_single_list() {
    let _sandbox = ControllerPersistenceSandbox::new();
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
    controller.select_source_by_index(0);
    controller.select_source_by_index_in_pane(FolderPaneId::Lower, 1);

    controller.apply_native_ui_action(NativeUiAction::RemoveSourceRow { index: 1 });

    assert_eq!(controller.active_folder_pane(), FolderPaneId::Upper);
    assert_eq!(controller.ui.sources.rows.len(), 1);
    assert_eq!(
        controller.ui.sources.rows[0].path,
        source_a.to_string_lossy()
    );
}

#[test]
/// Focusing a source row selects from the visible list and focuses sources.
fn focus_source_row_action_selects_single_active_source_and_focuses_sources_list() {
    let _sandbox = ControllerPersistenceSandbox::new();
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let source_a = dir.path().join("source-a");
    let source_b = dir.path().join("source-b");
    std::fs::create_dir_all(&source_a).unwrap();
    std::fs::create_dir_all(&source_b).unwrap();
    controller.add_source_from_path(source_a).unwrap();
    controller.add_source_from_path(source_b).unwrap();
    controller.select_source_by_index(0);
    let source_b_id = controller
        .source_id_for_index(1)
        .expect("source-b id should exist");
    controller.ui.focus.context = FocusContext::Waveform;

    controller.apply_native_ui_action(NativeUiAction::FocusSourceRow { index: 1 });

    assert_eq!(controller.active_folder_pane(), FolderPaneId::Upper);
    assert_eq!(
        controller.folder_pane_source(FolderPaneId::Upper),
        Some(source_b_id)
    );
    assert_eq!(controller.ui.sources.selected, Some(1));
    assert_eq!(controller.ui.focus.context, FocusContext::SourcesList);
}

#[test]
fn focus_sources_panel_preserves_selected_source_row() {
    let _sandbox = ControllerPersistenceSandbox::new();
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
