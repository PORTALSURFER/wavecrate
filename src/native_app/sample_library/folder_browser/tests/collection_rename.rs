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
