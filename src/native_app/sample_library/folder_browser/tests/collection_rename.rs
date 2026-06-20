use super::*;

#[test]
fn repeated_collection_activation_does_not_start_rename() {
    let root = temp_source_root("wavecrate-gui-collection-slow-click");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");

    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert!(browser.collection_rename_view(collection).is_none());

    browser.apply_message(FolderBrowserMessage::RenameCollection(collection));

    assert!(browser.collection_rename_view(collection).is_some());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn collection_rename_view_selects_full_collection_name() {
    let root = temp_source_root("wavecrate-gui-collection-rename-select-all");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");

    browser.apply_message(FolderBrowserMessage::RenameCollection(collection));
    let rename = browser
        .collection_rename_view(collection)
        .expect("collection rename view");

    assert_eq!(rename.draft, "Collection 1");
    assert_eq!(rename.selection_start, 0);
    assert_eq!(rename.selection_end, "Collection 1".chars().count());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn cancel_rename_exits_collection_rename() {
    let root = temp_source_root("wavecrate-gui-collection-cancel-rename");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");
    browser.apply_message(FolderBrowserMessage::RenameCollection(collection));
    assert!(browser.collection_rename_view(collection).is_some());

    browser.apply_message(FolderBrowserMessage::CancelRename);

    assert!(browser.collection_rename_view(collection).is_none());
    assert!(!browser.rename_active());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn collection_rename_exports_and_applies_custom_name_settings() {
    let root = temp_source_root("wavecrate-gui-collection-rename-settings");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");

    browser.apply_message(FolderBrowserMessage::RenameCollection(collection));
    let result = submit_rename(&mut browser, "Drums");
    assert_eq!(result.status, "Collection renamed");

    let saved_names = browser.custom_collection_names();
    assert_eq!(saved_names.get("0").map(String::as_str), Some("Drums"));

    let mut reloaded = FolderBrowserState::from_root(root.clone());
    reloaded.apply_collection_names(&saved_names);

    assert_eq!(
        reloaded
            .visible_collections()
            .into_iter()
            .find(|entry| entry.collection == collection)
            .map(|entry| entry.name),
        Some(String::from("Drums"))
    );
    let _ = fs::remove_dir_all(root);
}
