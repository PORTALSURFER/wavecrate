use super::*;

#[test]
fn folder_lock_marks_folder_and_descendants() {
    let root = temp_source_root("wavecrate-gui-folder-lock-state");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create kicks");
    fs::write(kicks.join("kick.wav"), [0_u8; 8]).expect("write kick");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let drums_id = path_id(&drums);
    let kicks_id = path_id(&kicks);

    assert_eq!(browser.toggle_folder_lock(&drums_id), Ok(true));
    browser.activate_folder(drums_id.clone());
    browser.expand_selected_folder();

    let visible = browser.visible_folders();
    let drums_row = visible
        .iter()
        .find(|folder| folder.id == drums_id)
        .expect("drums row");
    let kicks_row = visible
        .iter()
        .find(|folder| folder.id == kicks_id)
        .expect("kicks row");
    assert!(drums_row.locked);
    assert!(!drums_row.lock_inherited);
    assert!(kicks_row.locked);
    assert!(kicks_row.lock_inherited);
    assert_eq!(browser.locked_folder_paths(), vec![drums]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_lock_blocks_moving_locked_subtree() {
    let root = temp_source_root("wavecrate-gui-folder-lock-folder-move");
    let kicks = root.join("drums").join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks");
    fs::create_dir_all(&loops).expect("create loops");
    fs::write(kicks.join("kick.wav"), [0_u8; 8]).expect("write kick");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let kicks_id = path_id(&kicks);
    let loops_id = path_id(&loops);
    assert_eq!(browser.toggle_folder_lock(&kicks_id), Ok(true));

    browser.apply_folder_drag(kicks_id, DragHandleMessage::started(Point::new(0.0, 0.0)));
    let result = submit_folder_drop(&mut browser, &loops_id).expect("drop result");

    assert_eq!(result.status.as_deref(), Some("Drop target unchanged"));
    assert!(kicks.join("kick.wav").is_file());
    assert!(!loops.join("kicks").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_lock_blocks_file_move_into_locked_target() {
    let root = temp_source_root("wavecrate-gui-folder-lock-file-target");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums");
    fs::create_dir_all(&loops).expect("create loops");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write kick");
    fs::write(loops.join("loop.wav"), [1_u8; 8]).expect("write loop");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let kick_id = path_id(&kick);
    let drums_id = path_id(&drums);
    let loops_id = path_id(&loops);
    assert_eq!(browser.toggle_folder_lock(&loops_id), Ok(true));

    browser.activate_folder(drums_id);
    browser.select_file(kick_id.clone());
    browser.begin_file_drag(kick_id, Point::new(0.0, 0.0));
    let result = submit_folder_drop(&mut browser, &loops_id).expect("drop result");

    assert_eq!(result.status.as_deref(), Some("Drop target unchanged"));
    assert!(kick.is_file());
    assert!(!loops.join("kick.wav").exists());
    let _ = fs::remove_dir_all(root);
}
