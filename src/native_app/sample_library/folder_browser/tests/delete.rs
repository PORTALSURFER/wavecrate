use super::*;

#[test]
fn folder_delete_blocks_hard_delete_and_keeps_selected_folder() {
    let root = temp_source_root("wavecrate-gui-folder-delete");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create nested folder");
    fs::write(kicks.join("kick.wav"), [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.expand_selected_folder();
    browser.activate_folder(path_id(&kicks));

    let target = browser
        .selected_delete_target()
        .expect("subfolder can be deleted");
    assert_eq!(target.name, "kicks");
    let error = browser
        .delete_selected_folder()
        .expect_err("hard delete should be blocked");

    assert_eq!(
        error,
        "Trash workflow is not available in the default GUI yet; no folder was deleted"
    );
    assert!(kicks.exists());
    assert_eq!(browser.selection.selected_folder, path_id(&kicks));
    assert!(browser.find_folder(&path_id(&kicks)).is_some());
    assert!(browser.tree.expanded_folders.contains(&path_id(&drums)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn trashed_selected_folder_is_removed_from_active_tree_without_parked_source_cache() {
    let root = temp_source_root("wavecrate-gui-folder-trash-active-tree");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create nested folder");
    fs::write(kicks.join("kick.wav"), [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.expand_selected_folder();
    browser.activate_folder(path_id(&kicks));
    browser.source.sources[0].root_folder = None;

    assert!(browser.discard_trashed_folder_path(&kicks));

    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert!(browser.find_folder(&path_id(&kicks)).is_none());
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .all(|folder| folder.id != path_id(&kicks)),
        "trashed folder should disappear from the visible folder tree immediately"
    );
    assert!(
        !browser.tree.expanded_folders.contains(&path_id(&kicks)),
        "trashed folder expansion state should be discarded"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_delete_blocks_hard_delete_and_keeps_selection() {
    let root = temp_source_root("wavecrate-gui-file-delete");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    for file in [&hat, &kick, &snare] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));

    let target = browser
        .selected_file_delete_target()
        .expect("selected file can be deleted");
    assert_eq!(target.names, vec![String::from("kick.wav")]);
    let error = browser
        .delete_selected_files()
        .expect_err("hard delete should be blocked");

    assert_eq!(
        error,
        "Trash workflow is not available in the default GUI yet; no files were deleted"
    );
    assert!(kick.exists());
    assert!(
        browser
            .selected_files()
            .iter()
            .any(|file| file.name == "kick.wav")
    );
    assert_eq!(browser.selected_file_id(), Some(path_id(&kick).as_str()));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn trashed_selected_file_focuses_next_sample_without_clearing_file_selection() {
    let root = temp_source_root("wavecrate-gui-file-trash-focus-next");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    for file in [&hat, &kick, &snare] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));

    assert!(browser.discard_trashed_file_paths(std::slice::from_ref(&kick)));

    assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));
    assert_eq!(browser.selected_file_paths(), vec![snare.clone()]);
    assert!(
        !browser
            .selected_files()
            .iter()
            .any(|file| file.id == path_id(&kick))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn trashed_last_selected_file_focuses_previous_sample() {
    let root = temp_source_root("wavecrate-gui-file-trash-focus-previous");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    for file in [&hat, &kick, &snare] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&snare));

    assert!(browser.discard_trashed_file_paths(std::slice::from_ref(&snare)));

    assert_eq!(browser.selected_file_id(), Some(path_id(&kick).as_str()));
    assert_eq!(browser.selected_file_paths(), vec![kick.clone()]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn trashed_explicit_file_selection_focuses_next_sample_after_focused_row() {
    let root = temp_source_root("wavecrate-gui-file-trash-focus-explicit");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let tom = drums.join("tom.wav");
    for file in [&hat, &kick, &snare, &tom] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&hat));
    browser.select_file_with_modifiers(
        path_id(&kick),
        PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        },
    );

    assert!(browser.discard_trashed_file_paths(&[hat.clone(), kick.clone()]));

    assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));
    assert_eq!(browser.selected_file_paths(), vec![snare.clone()]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn root_folder_delete_is_rejected_from_tree() {
    let root = temp_source_root("wavecrate-gui-root-delete");
    let browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(
        browser.selected_delete_target(),
        Err(String::from("Root folder cannot be deleted"))
    );
    assert!(root.is_dir());
    let _ = fs::remove_dir_all(root);
}
