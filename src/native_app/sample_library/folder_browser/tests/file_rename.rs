use super::*;
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

    let status = submit_rename(&mut browser, "snare loop").status;

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
fn file_rename_updates_active_tree_without_parked_source_cache() {
    let root = temp_source_root("wavecrate-gui-file-rename-active-tree");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    let kick = drums.join("kick loop.wav");
    let snare = drums.join("snare loop.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));
    browser.source.sources[0].root_folder = None;
    browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input id");

    let status = submit_rename(&mut browser, "snare loop").status;

    assert_eq!(status, "Renamed file to snare loop.wav");
    assert!(!kick.exists());
    assert!(snare.is_file());
    assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));
    assert!(
        browser
            .selected_audio_files()
            .iter()
            .all(|file| file.id != path_id(&kick)),
        "renamed file should disappear from the old active-tree path"
    );
    assert!(
        browser
            .selected_audio_files()
            .iter()
            .any(|file| file.id == path_id(&snare)),
        "renamed file should appear at the new active-tree path"
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

    let status = submit_rename(&mut browser, "snare.aiff").status;

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
    let db = wavecrate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(&root)
        .expect("db");
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

    let status = submit_rename(&mut browser, "snare").status;

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
