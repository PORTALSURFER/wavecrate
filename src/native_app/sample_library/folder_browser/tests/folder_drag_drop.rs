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
        DragHandleMessage::started(Point::new(0.0, 0.0)),
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
    assert_eq!(browser.selection.selected_folder, path_id(&moved_kicks));
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&moved_kick).as_str())
    );
    assert!(browser.find_folder(&path_id(&moved_kicks)).is_some());
    assert!(browser.tree.expanded_folders.contains(&path_id(&loops)));
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
        DragHandleMessage::started(Point::new(10.0, 20.0)),
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
        DragHandleMessage::moved(Point::new(30.0, 42.0)),
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

    let revision_before_exit = browser.drag_drop.revision();
    browser.apply_message(FolderBrowserMessage::ClearDropTarget(Point::new(
        90.0, 120.0,
    )));
    assert_eq!(
        browser.drag_drop.revision(),
        revision_before_exit + 1,
        "clearing a folder drop target must refresh retained folder hit targets"
    );
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .all(|folder| !folder.drop_target)
    );

    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(50.0, 60.0),
    ));
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .any(|folder| folder.drop_target)
    );

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
        DragHandleMessage::started(Point::new(10.0, 20.0)),
    );
    assert_eq!(browser.external_drag_request(), None);
    let _ = fs::remove_dir_all(root);
}
