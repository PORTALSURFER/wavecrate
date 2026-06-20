use super::*;

#[test]
fn escape_collection_focus_restores_previous_folder_focus() {
    let root = temp_source_root("wavecrate-gui-collection-escape-restore");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let alpha_keep = alpha.join("alpha_keep.wav");
    let beta_keep = beta.join("beta_keep.wav");
    fs::write(&alpha_keep, []).expect("write alpha sample");
    fs::write(&beta_keep, []).expect("write beta sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&beta));

    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&alpha_keep, collection);
    browser.set_file_collection_state(&beta_keep, collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha_keep.wav", "beta_keep.wav"]
    );

    browser.apply_message(FolderBrowserMessage::ExitCollectionFocus);

    assert_eq!(browser.selection.selected_collection, None);
    assert_eq!(browser.selected_folder_path(), Some(beta.clone()));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["beta_keep.wav"]
    );
    assert_eq!(
        browser
            .visible_folders()
            .into_iter()
            .find(|folder| folder.id == path_id(&beta))
            .map(|folder| folder.selected),
        Some(true)
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn escape_collection_focus_without_valid_prior_folder_clears_collection() {
    let root = temp_source_root("wavecrate-gui-collection-escape-no-prior");
    let alpha = root.join("alpha");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    let keep = alpha.join("keep.wav");
    fs::write(&keep, []).expect("write sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.selection.selected_folder = String::from("missing-folder");

    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&keep, collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(browser.selected_folder_path(), None);

    browser.apply_message(FolderBrowserMessage::ExitCollectionFocus);

    assert_eq!(browser.selection.selected_collection, None);
    assert_eq!(browser.selected_folder_path(), Some(root.clone()));
    let _ = fs::remove_dir_all(root);
}
#[test]
fn collection_keyboard_navigation_moves_between_collections_without_entering_files() {
    let root = temp_source_root("wavecrate-gui-collection-keyboard");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let alpha_keep = alpha.join("alpha_keep.wav");
    let beta_keep = beta.join("beta_keep.wav");
    fs::write(&alpha_keep, []).expect("write alpha sample");
    fs::write(&beta_keep, []).expect("write beta sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&beta));
    let previous_collection = SampleCollection::new(0).expect("previous collection");
    let collection = SampleCollection::new(1).expect("collection");
    let next_collection = SampleCollection::new(2).expect("next collection");
    browser.set_file_collection_state(&alpha_keep, collection);
    browser.set_file_collection_state(&beta_keep, next_collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(browser.navigate_vertical(1, false), None);
    assert_eq!(browser.selection.selected_collection, Some(next_collection));
    assert_eq!(browser.selected_file_id(), None);
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["beta_keep.wav"]
    );

    assert_eq!(browser.navigate_vertical(-1, false), None);
    assert_eq!(browser.selection.selected_collection, Some(collection));
    assert_eq!(browser.selected_file_id(), None);
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha_keep.wav"]
    );

    assert_eq!(browser.navigate_vertical(-1, false), None);
    assert_eq!(
        browser.selection.selected_collection,
        Some(previous_collection)
    );
    assert_eq!(browser.navigate_vertical(-1, false), None);
    assert_eq!(
        browser.selection.selected_collection,
        Some(previous_collection)
    );
    assert_eq!(browser.selected_folder_path(), None);
    assert!(!browser.expand_selected_folder());
    assert!(!browser.collapse_selected_folder());

    let _ = fs::remove_dir_all(root);
}
