use super::*;

#[test]
fn browser_keyboard_navigation_is_disabled_while_renaming() {
    let root = temp_source_root("wavecrate-gui-keyboard-rename");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input id");

    assert!(browser.rename_active());
    assert_eq!(browser.navigate_vertical(1, false), None);
    assert!(!browser.expand_selected_folder());
    assert!(!browser.collapse_selected_folder());
    assert_eq!(browser.selected_folder, path_id(&drums));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_rename_updates_filesystem_tree_and_selected_audio_files() {
    let root = temp_source_root("wavecrate-gui-folder-rename");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    let input_id = browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input id");
    assert_ne!(input_id, 0);
    let status = browser
        .apply_rename_input(TextInputMessage::Submitted {
            value: String::from("breaks"),
        })
        .expect("rename status")
        .status;

    assert_eq!(status, "Renamed folder to breaks");
    assert!(!drums.exists());
    assert!(root.join("breaks").join("kick.wav").is_file());
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.id.as_str())
            .collect::<Vec<_>>(),
        vec![path_id(&root.join("breaks").join("kick.wav"))]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn create_subfolder_starts_pending_rename_row_and_creates_on_submit() {
    let root = temp_source_root("wavecrate-gui-folder-create");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    let input_id = browser
        .begin_create_subfolder()
        .expect("create can start")
        .expect("rename input id");
    let pending = drums.join("New folder");

    assert_ne!(input_id, 0);
    assert!(!pending.exists());
    assert!(browser.is_expanded(&path_id(&drums)));
    assert!(
        browser.visible_folders().into_iter().any(|folder| {
            folder.id == path_id(&pending)
                && folder.selected
                && folder.rename_draft.as_deref() == Some("New folder")
                && folder.rename_input_id == Some(input_id)
        }),
        "expected pending child rename row"
    );

    let status = browser
        .apply_rename_input(TextInputMessage::Submitted {
            value: String::from("loops"),
        })
        .expect("create status")
        .status;

    assert_eq!(status, "Created folder loops");
    assert!(!pending.exists());
    assert!(drums.join("loops").is_dir());
    assert_eq!(browser.selected_folder, path_id(&drums.join("loops")));
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .any(|folder| folder.id == path_id(&drums.join("loops"))
                && folder.name == "loops"
                && folder.rename_draft.is_none())
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn create_subfolder_cancel_removes_pending_row_without_touching_disk() {
    let root = temp_source_root("wavecrate-gui-folder-create-cancel");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser
        .begin_create_subfolder()
        .expect("create can start")
        .expect("rename input id");
    let pending = drums.join("New folder");

    browser.activate_folder(path_id(&drums));

    assert!(!pending.exists());
    assert_eq!(browser.selected_folder, path_id(&drums));
    assert!(
        browser
            .visible_folders()
            .into_iter()
            .all(|folder| folder.id != path_id(&pending))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn create_subfolder_default_name_skips_existing_folder() {
    let root = temp_source_root("wavecrate-gui-folder-create-unique");
    let drums = root.join("drums");
    fs::create_dir_all(drums.join("New folder")).expect("create existing folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser
        .begin_create_subfolder()
        .expect("create can start")
        .expect("rename input id");

    assert!(
        browser.visible_folders().into_iter().any(|folder| {
            folder.id == path_id(&drums.join("New folder 2"))
                && folder.rename_draft.as_deref() == Some("New folder 2")
        }),
        "expected unique default name"
    );
    assert!(!drums.join("New folder 2").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_rename_hides_and_preserves_extension() {
    let root = temp_source_root("wavecrate-gui-file-rename");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    let kick = drums.join("kick loop.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));

    let input_id = browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input id");
    let rename = browser
        .file_rename_view(&path_id(&kick))
        .expect("file rename view");
    assert_eq!(rename.input_id, input_id);
    assert_eq!(rename.draft, "kick loop");
    assert_eq!(rename.selection_start, 0);
    assert_eq!(rename.selection_end, "kick loop".chars().count());

    let status = browser
        .apply_rename_input(TextInputMessage::Submitted {
            value: String::from("snare loop"),
        })
        .expect("rename status")
        .status;

    assert_eq!(status, "Renamed file to snare loop.wav");
    assert!(!kick.exists());
    assert!(drums.join("snare loop.wav").is_file());
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&drums.join("snare loop.wav")).as_str())
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| (
                file.name.as_str(),
                file.stem.as_str(),
                file.extension.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![("snare loop.wav", "snare loop", "wav")]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_rename_submission_cannot_change_extension() {
    let root = temp_source_root("wavecrate-gui-file-rename-extension");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));
    browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input id");

    let status = browser
        .apply_rename_input(TextInputMessage::Submitted {
            value: String::from("snare.aiff"),
        })
        .expect("rename status")
        .status;

    assert_eq!(status, "Renamed file to snare.aiff.wav");
    assert!(!kick.exists());
    assert!(drums.join("snare.aiff.wav").is_file());
    assert!(!drums.join("snare.aiff").exists());
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&drums.join("snare.aiff.wav")).as_str())
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_rename_keeps_active_collection_item_visible_and_persisted() {
    let root = temp_source_root("wavecrate-gui-file-rename-collection");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let collection = SampleCollection::new(1).expect("collection");
    let db = wavecrate::sample_sources::SourceDatabase::open(&root).expect("db");
    db.upsert_file(std::path::Path::new("drums/kick.wav"), 8, 5)
        .expect("upsert file");
    let mut batch = db.write_batch().expect("write batch");
    batch
        .add_collection(std::path::Path::new("drums/kick.wav"), collection)
        .expect("add collection");
    batch.commit().expect("commit collection");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));
    browser.select_file(path_id(&kick));
    browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input id");

    let status = browser
        .apply_rename_input(TextInputMessage::Submitted {
            value: String::from("snare"),
        })
        .expect("rename status")
        .status;

    assert_eq!(status, "Renamed file to snare.wav");
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| (file.name.as_str(), file.belongs_to_collection(collection)))
            .collect::<Vec<_>>(),
        vec![("snare.wav", true)]
    );
    assert_eq!(
        db.collections_for_path(std::path::Path::new("drums/kick.wav"))
            .expect("old collections"),
        Vec::<SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(std::path::Path::new("drums/snare.wav"))
            .expect("new collections"),
        vec![collection]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn root_folder_rename_is_rejected_from_tree() {
    let root = temp_source_root("wavecrate-gui-root-rename");
    let mut browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(
        browser.begin_rename_selected(),
        Err(String::from("Select a subfolder to rename"))
    );
    assert!(root.is_dir());
    let _ = fs::remove_dir_all(root);
}
