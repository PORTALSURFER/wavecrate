use super::*;
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
fn file_drag_hover_remains_valid_when_selected_projection_hides_file() {
    let root = temp_source_root("wavecrate-gui-file-drag-hover-source-tree");
    let drums = root.join("drums");
    let target = root.join("target");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&target).expect("create target folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    browser.filters.name_filter = String::from("snare");
    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&target),
        Point::new(50.0, 60.0),
    ));

    let hovered = browser
        .visible_folders()
        .into_iter()
        .find(|folder| folder.id == path_id(&target))
        .expect("target folder visible");
    assert!(
        hovered.drop_target,
        "folder drop highlight should follow the captured drag payload, not the current selected folder projection"
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn clear_drop_target_unless_preserves_current_folder_target() {
    let root = temp_source_root("wavecrate-gui-file-drag-hover-preserve");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(50.0, 60.0),
    ));
    browser.apply_message(FolderBrowserMessage::ClearDropTargetUnless(
        path_id(&loops),
        Point::new(51.0, 61.0),
    ));

    assert_eq!(
        browser.hovered_drop_target_folder_id(),
        Some(path_id(&loops)),
        "stale row clear messages must not erase the active folder target while the pointer is still on it"
    );
    browser.apply_message(FolderBrowserMessage::ClearDropTargetUnless(
        path_id(&drums),
        Point::new(52.0, 62.0),
    ));
    assert_eq!(browser.hovered_drop_target_folder_id(), None);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_drag_hover_moves_between_folder_targets() {
    let root = temp_source_root("wavecrate-gui-file-drag-target-handoff");
    let drums = root.join("drums");
    let loops = root.join("loops");
    let one_shots = root.join("one-shots");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    fs::create_dir_all(&one_shots).expect("create one-shots folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&loops),
        Point::new(50.0, 60.0),
    ));
    browser.apply_message(FolderBrowserMessage::HoverDropTarget(
        path_id(&one_shots),
        Point::new(50.0, 82.0),
    ));

    assert_eq!(
        browser.hovered_drop_target_folder_id(),
        Some(path_id(&one_shots))
    );
    let folders = browser.visible_folders();
    assert!(
        folders
            .iter()
            .any(|folder| folder.id == path_id(&one_shots) && folder.drop_target)
    );
    assert!(
        folders
            .iter()
            .all(|folder| folder.id != path_id(&loops) || !folder.drop_target)
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
