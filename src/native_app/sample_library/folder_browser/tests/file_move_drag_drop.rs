use super::*;
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
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert_eq!(browser.selected_file_paths(), vec![hat.clone()]);
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["hat.wav"]
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_drag_drop_defers_destination_name_conflicts() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let existing_kick = loops.join("kick.wav");
    for file in [&kick, &snare, &existing_kick] {
        fs::write(file, file.display().to_string()).expect("write wav");
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
        .expect("non-conflicting files should still move");

    let moved_snare = loops.join("snare.wav");
    assert_eq!(
        result.moved_paths,
        vec![(snare.clone(), moved_snare.clone())]
    );
    assert!(kick.is_file());
    assert!(!snare.exists());
    assert!(moved_snare.is_file());
    assert_eq!(browser.pending_file_move_conflict_count(), 1);
    let conflict = browser
        .pending_file_move_conflict_view()
        .expect("conflict dialog state");
    assert_eq!(conflict.file_name, "kick.wav");
    assert_eq!(conflict.source_path, kick);
    assert_eq!(conflict.destination_path, existing_kick);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_move_conflict_rename_uses_available_copy_name() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-rename");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let existing = loops.join("kick.wav");
    let first_copy = loops.join("kick_copy001.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&existing, b"existing").expect("write existing");
    fs::write(&first_copy, b"copy").expect("write copy");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&source));

    browser.begin_file_drag(path_id(&source), Point::new(4.0, 8.0));
    browser
        .drop_drag_on_folder(&path_id(&loops))
        .expect("drop should park conflict");
    let result = browser
        .resolve_next_file_move_conflict(FileMoveConflictResolution::Rename)
        .expect("rename conflict should move source");

    let renamed = loops.join("kick_copy002.wav");
    assert_eq!(result.moved_paths, vec![(source.clone(), renamed.clone())]);
    assert!(!source.exists());
    assert_eq!(fs::read(&existing).expect("read existing"), b"existing");
    assert_eq!(fs::read(&renamed).expect("read renamed"), b"source");
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    assert_eq!(browser.selected_file_paths(), vec![renamed]);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_move_conflict_overwrite_replaces_destination() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-overwrite");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let destination = loops.join("kick.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&destination, b"destination").expect("write destination");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&source));

    browser.begin_file_drag(path_id(&source), Point::new(4.0, 8.0));
    browser
        .drop_drag_on_folder(&path_id(&loops))
        .expect("drop should park conflict");
    let result = browser
        .resolve_next_file_move_conflict(FileMoveConflictResolution::Overwrite)
        .expect("overwrite conflict should move source");

    assert_eq!(
        result.moved_paths,
        vec![(source.clone(), destination.clone())]
    );
    assert!(!source.exists());
    assert_eq!(fs::read(&destination).expect("read destination"), b"source");
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    assert_eq!(browser.selected_file_paths(), vec![destination]);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_move_conflict_skip_leaves_source_and_destination() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-skip");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let destination = loops.join("kick.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&destination, b"destination").expect("write destination");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&source));

    browser.begin_file_drag(path_id(&source), Point::new(4.0, 8.0));
    browser
        .drop_drag_on_folder(&path_id(&loops))
        .expect("drop should park conflict");
    let result = browser
        .resolve_next_file_move_conflict(FileMoveConflictResolution::Skip)
        .expect("skip conflict should succeed");

    assert!(result.moved_paths.is_empty());
    assert_eq!(fs::read(&source).expect("read source"), b"source");
    assert_eq!(
        fs::read(&destination).expect("read destination"),
        b"destination"
    );
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    let _ = fs::remove_dir_all(root);
}
