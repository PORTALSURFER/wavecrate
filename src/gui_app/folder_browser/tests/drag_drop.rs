use super::*;

#[test]
fn folder_drag_drop_moves_subtree_into_target_folder() {
    let root = temp_source_root("wavecrate-gui-folder-drag-drop");
    let kicks = root.join("drums").join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = kicks.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&root.join("drums")));
    browser.expand_selected_folder();
    browser.activate_folder(path_id(&kicks));
    browser.select_file(path_id(&kick));

    browser.apply_folder_drag(
        path_id(&kicks),
        DragHandleMessage::Started {
            position: Point::new(0.0, 0.0),
        },
    );
    let result = browser
        .drop_drag_on_folder(&path_id(&loops))
        .expect("folder drag/drop should move");

    let moved_kicks = loops.join("kicks");
    let moved_kick = moved_kicks.join("kick.wav");
    assert_eq!(
        result.moved_paths,
        vec![(kicks.clone(), moved_kicks.clone())]
    );
    assert!(!kicks.exists());
    assert!(moved_kick.is_file());
    assert_eq!(browser.selected_folder, path_id(&moved_kicks));
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&moved_kick).as_str())
    );
    assert!(browser.find_folder(&path_id(&moved_kicks)).is_some());
    assert!(browser.expanded_folders.contains(&path_id(&loops)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_drag_preview_tracks_pointer_and_hover_target() {
    let root = temp_source_root("wavecrate-gui-folder-drag-preview");
    let kicks = root.join("drums").join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&root.join("drums")));
    browser.expand_selected_folder();

    browser.apply_folder_drag(
        path_id(&kicks),
        DragHandleMessage::Started {
            position: Point::new(10.0, 20.0),
        },
    );
    assert_eq!(
        browser.drag_preview(),
        Some(FolderDragPreview {
            label: String::from("kicks"),
            pointer: Point::new(10.0, 20.0),
        })
    );

    browser.apply_folder_drag(
        path_id(&kicks),
        DragHandleMessage::Moved {
            position: Point::new(30.0, 42.0),
        },
    );
    assert_eq!(
        browser.drag_preview().map(|preview| preview.pointer),
        Some(Point::new(30.0, 42.0))
    );

    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(50.0, 60.0),
    ));
    assert_eq!(
        browser.drag_preview().map(|preview| preview.pointer),
        Some(Point::new(50.0, 60.0))
    );
    let hovered = browser
        .visible_folders()
        .into_iter()
        .find(|folder| folder.id == path_id(&loops))
        .expect("loops folder visible");
    assert!(hovered.drop_candidate);
    assert!(hovered.drop_target);

    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&kicks),
        Point::new(70.0, 80.0),
    ));
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .all(|folder| !folder.drop_target)
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_drag_does_not_arm_external_file_drag() {
    let root = temp_source_root("wavecrate-gui-folder-no-external-drag");
    let kicks = root.join("drums").join("kicks");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&root.join("drums")));
    browser.expand_selected_folder();

    browser.apply_folder_drag(
        path_id(&kicks),
        DragHandleMessage::Started {
            position: Point::new(10.0, 20.0),
        },
    );
    assert_eq!(browser.external_drag_request(), None);
    let _ = fs::remove_dir_all(root);
}

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
    let result = browser
        .drop_drag_on_folder(&path_id(&loops))
        .expect("extracted file drag/drop should move");

    let moved = loops.join("loop_extraction.wav");
    assert_eq!(result.moved_paths, vec![(extracted.clone(), moved.clone())]);
    assert!(!extracted.exists());
    assert!(original.is_file());
    assert!(moved.is_file());
    assert_eq!(browser.selected_folder, path_id(&drums));
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

    let initial_revision = browser.drag_revision();
    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    assert_eq!(
        browser.drag_revision(),
        initial_revision,
        "starting a drag should keep existing row widget state until the drag is cleared"
    );

    browser.clear_drag();
    assert_eq!(
        browser.drag_revision(),
        initial_revision + 1,
        "clearing a drag must refresh retained sample-row hit targets so stale pressed/drag paint cannot survive cancellation"
    );

    browser.clear_drag();
    assert_eq!(
        browser.drag_revision(),
        initial_revision + 1,
        "clearing already-idle drag state should not churn row identity"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn clearing_collection_drop_target_advances_drag_revision() {
    let root = temp_source_root("wavecrate-gui-collection-drag-revision");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    browser.apply_message(FolderBrowserMessage::HoverCollectionDropTarget(
        SampleCollection::new(0).unwrap(),
        Point::new(12.0, 18.0),
    ));
    assert!(
        browser
            .visible_collections()
            .into_iter()
            .any(|collection| collection.drop_target),
        "hovering a collection during file drag should mark the collection drop target"
    );

    let revision_before_clear = browser.drag_revision();
    browser.clear_drag();

    assert_eq!(
        browser.drag_revision(),
        revision_before_clear + 1,
        "clearing a collection drop target must refresh retained drag widgets"
    );
    assert!(browser.drag_preview().is_none());
    assert!(
        browser
            .visible_collections()
            .into_iter()
            .all(|collection| !collection.drop_target)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_moves_selected_files_into_target_folder() {
    let root = temp_source_root("wavecrate-gui-file-drag-drop");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
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
    let result = browser
        .drop_drag_on_folder(&path_id(&loops))
        .expect("file drag/drop should move");

    let moved_kick = loops.join("kick.wav");
    let moved_snare = loops.join("snare.wav");
    assert_eq!(result.moved_paths.len(), 2);
    assert!(!kick.exists());
    assert!(!snare.exists());
    assert!(hat.is_file());
    assert!(moved_kick.is_file());
    assert!(moved_snare.is_file());
    assert_eq!(browser.selected_folder, path_id(&loops));
    assert_eq!(
        browser.selected_file_paths(),
        vec![moved_kick.clone(), moved_snare.clone()]
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav", "snare.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_hover_uses_cached_file_entry_without_filesystem_probe() {
    let root = temp_source_root("wavecrate-gui-file-drag-hover-cached");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    fs::remove_file(&kick).expect("remove dragged file after browser cached it");
    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(50.0, 60.0),
    ));

    let hovered = browser
        .visible_folders()
        .into_iter()
        .find(|folder| folder.id == path_id(&loops))
        .expect("loops folder visible");
    assert!(
        hovered.drop_target,
        "drag hover should not depend on probing the dragged file while it may be busy loading"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_hover_without_active_drag_does_not_mark_drop_target() {
    let root = temp_source_root("wavecrate-gui-folder-hover-no-drag");
    let loops = root.join("loops");
    fs::create_dir_all(&loops).expect("create loops folder");
    let mut browser = FolderBrowserState::from_root(root.clone());

    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(50.0, 60.0),
    ));

    assert!(
        browser
            .visible_folders()
            .into_iter()
            .all(|folder| !folder.drop_target),
        "optimistic hover messages should remain harmless when no drag is active"
    );
    let _ = fs::remove_dir_all(root);
}
