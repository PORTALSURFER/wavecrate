use super::*;
#[test]
fn file_drag_external_request_uses_selected_file_paths() {
    let root = temp_source_root("wavecrate-gui-file-external-drag");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let hat = drums.join("hat.wav");
    for file in [&kick, &snare, &hat] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));
    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    let request = browser
        .external_drag_request()
        .expect("file drag should expose external request");

    assert_eq!(request.preview.label, "2 files");
    assert_eq!(
        request.payload,
        ExternalDragPayload::Files(vec![kick.clone(), snare.clone()])
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn extracted_file_drag_external_request_uses_extracted_path() {
    let root = temp_source_root("wavecrate-gui-extracted-file-external-drag");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let extracted = drums.join("loop_extraction.wav");
    fs::write(&extracted, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.refresh_file_path(&extracted);

    browser.begin_extracted_file_drag(extracted.clone(), Point::new(4.0, 8.0));
    let request = browser
        .external_drag_request()
        .expect("extracted file drag should expose external request");

    assert_eq!(request.preview.label, "loop_extraction.wav");
    assert_eq!(
        request.payload,
        ExternalDragPayload::Files(vec![extracted.clone()])
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn extracted_file_drag_drop_moves_file_into_target_folder() {
    let root = temp_source_root("wavecrate-gui-extracted-file-drag-drop");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let original = drums.join("loop.wav");
    let extracted = drums.join("loop_extraction.wav");
    fs::write(&original, [0_u8; 8]).expect("write wav");
    fs::write(&extracted, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.refresh_file_path(&extracted);
    browser.select_file(path_id(&original));

    browser.begin_extracted_file_drag(extracted.clone(), Point::new(4.0, 8.0));
    let result = submit_folder_drop(&mut browser, &path_id(&loops))
        .expect("extracted file drag/drop should move");

    let moved = loops.join("loop_extraction.wav");
    assert_eq!(result.moved_paths, vec![(extracted.clone(), moved.clone())]);
    assert!(!extracted.exists());
    assert!(original.is_file());
    assert!(moved.is_file());
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert_eq!(browser.selected_file_paths(), vec![original.clone()]);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn clearing_file_drag_advances_drag_revision_for_retained_row_reset() {
    let root = temp_source_root("wavecrate-gui-file-drag-revision");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));

    let initial_revision = browser.drag_drop.revision();
    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    assert_eq!(
        browser.drag_drop.revision(),
        initial_revision,
        "starting a drag should keep existing row widget state until the drag is cleared"
    );

    browser.clear_drag();
    assert_eq!(
        browser.drag_drop.revision(),
        initial_revision + 1,
        "clearing a drag must refresh retained sample-row hit targets so stale pressed/drag paint cannot survive cancellation"
    );

    browser.clear_drag();
    assert_eq!(
        browser.drag_drop.revision(),
        initial_revision + 1,
        "clearing already-idle drag state should not churn row identity"
    );
    let _ = fs::remove_dir_all(root);
}
