use super::*;

#[test]
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
