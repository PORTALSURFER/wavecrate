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
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("folder drag/drop should move");

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
fn folder_drag_drop_moves_all_selected_sibling_folders() {
    let root = temp_source_root("wavecrate-gui-folder-drag-drop-selected");
    let kicks = root.join("drums").join("kicks");
    let snares = root.join("drums").join("snares");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&snares).expect("create snares folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = kicks.join("kick.wav");
    let snare = snares.join("snare.wav");
    fs::write(&kick, [0_u8; 8]).expect("write kick");
    fs::write(&snare, [1_u8; 8]).expect("write snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&root.join("drums")));
    browser.expand_selected_folder();
    browser.activate_folder(path_id(&kicks));
    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        path_id(&snares),
        PointerModifiers {
            command: true,
            ..PointerModifiers::default()
        },
    ));

    browser.apply_folder_drag(
        path_id(&kicks),
        DragHandleMessage::started(Point::new(0.0, 0.0)),
    );
    assert_eq!(
        browser.drag_preview(),
        Some(FolderDragPreview {
            label: String::from("2 folders"),
            pointer: Point::new(0.0, 0.0),
        })
    );
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("folder drag/drop should move");

    let moved_kicks = loops.join("kicks");
    let moved_snares = loops.join("snares");
    assert_eq!(
        result.moved_paths,
        vec![
            (kicks.clone(), moved_kicks.clone()),
            (snares.clone(), moved_snares.clone())
        ]
    );
    assert!(!kicks.exists());
    assert!(!snares.exists());
    assert!(moved_kicks.join("kick.wav").is_file());
    assert!(moved_snares.join("snare.wav").is_file());
    assert_eq!(browser.selection.selected_folder, path_id(&moved_snares));
    assert_eq!(
        browser.selection.selected_folder_ids,
        std::collections::HashSet::from([path_id(&moved_kicks), path_id(&moved_snares)])
    );
    assert!(browser.tree.expanded_folders.contains(&path_id(&loops)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_drag_drop_preserves_nested_file_rating_after_reload() {
    let root = temp_source_root("wavecrate-gui-folder-drag-rating");
    let kicks = root.join("drums").join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = kicks.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let db = SourceDatabase::open(&root).expect("open source db");
    db.upsert_file(std::path::Path::new("drums/kicks/kick.wav"), 8, 1)
        .expect("upsert kick");
    db.set_tag(std::path::Path::new("drums/kicks/kick.wav"), Rating::new(2))
        .expect("set rating");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&root.join("drums")));
    browser.expand_selected_folder();
    browser.activate_folder(path_id(&kicks));

    browser.apply_folder_drag(
        path_id(&kicks),
        DragHandleMessage::started(Point::new(0.0, 0.0)),
    );
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("folder drag/drop should move");

    let moved_kicks = loops.join("kicks");
    let moved_kick = moved_kicks.join("kick.wav");
    assert_eq!(
        db.tag_for_path(std::path::Path::new("drums/kicks/kick.wav"))
            .expect("old rating"),
        None
    );
    assert_eq!(
        db.tag_for_path(std::path::Path::new("loops/kicks/kick.wav"))
            .expect("moved rating"),
        Some(Rating::new(2))
    );

    let mut reloaded = FolderBrowserState::from_root(root.clone());
    reloaded.activate_folder(path_id(&moved_kicks));
    let moved = reloaded
        .selected_audio_files()
        .into_iter()
        .find(|file| file.id == path_id(&moved_kick))
        .expect("moved kick row after reload");
    assert_eq!(moved.rating, Rating::new(2));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_drag_preview_tracks_pointer_and_hover_target() {
    let root = temp_source_root("wavecrate-gui-folder-drag-preview");
    let kicks = root.join("drums").join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    fs::write(kicks.join("kick.wav"), [0_u8; 8]).expect("write kick");
    fs::write(loops.join("loop.wav"), [0_u8; 8]).expect("write loop");
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
