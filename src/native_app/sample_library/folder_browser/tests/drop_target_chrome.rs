use super::*;
#[test]
fn clearing_collection_drop_target_clears_drag_preview_and_collection_target() {
    let root = temp_source_root("wavecrate-gui-collection-drag-clear");
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

    browser.clear_drag();

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
fn collection_hover_clears_folder_drop_target_during_file_drag() {
    let root = temp_source_root("wavecrate-gui-folder-to-collection-drop-target");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    fs::write(loops.join("loop.wav"), [0_u8; 8]).expect("write loop");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(12.0, 18.0),
    ));
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .any(|folder| folder.id == path_id(&loops) && folder.drop_target),
        "hovering a folder during file drag should mark the folder drop target"
    );

    browser.apply_message(FolderBrowserMessage::HoverCollectionDropTarget(
        SampleCollection::new(0).unwrap(),
        Point::new(24.0, 32.0),
    ));

    assert!(
        browser
            .visible_folders()
            .into_iter()
            .all(|folder| !folder.drop_target),
        "collection hover must clear the previous folder drop target"
    );
    assert!(
        browser
            .visible_collections()
            .into_iter()
            .any(|collection| collection.drop_target),
        "collection hover should keep the collection drop target active"
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn folder_hover_clears_collection_drop_target_during_file_drag() {
    let root = temp_source_root("wavecrate-gui-collection-to-folder-drop-target");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    fs::write(loops.join("loop.wav"), [0_u8; 8]).expect("write loop");
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

    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(24.0, 32.0),
    ));

    assert!(
        browser
            .visible_collections()
            .into_iter()
            .all(|collection| !collection.drop_target),
        "folder hover must clear the previous collection drop target"
    );
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .any(|folder| folder.id == path_id(&loops) && folder.drop_target),
        "folder hover should keep the folder drop target active"
    );
    let _ = fs::remove_dir_all(root);
}
