use super::*;

#[test]
fn activating_collection_filters_audio_files_across_selected_source() {
    let root = temp_source_root("wavecrate-gui-collection-filter");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let alpha_keep = alpha.join("alpha_keep.wav");
    let beta_keep = beta.join("beta_keep.wav");
    let beta_other = beta.join("beta_other.wav");
    fs::write(&alpha_keep, []).expect("write alpha sample");
    fs::write(&beta_keep, []).expect("write beta sample");
    fs::write(&beta_other, []).expect("write other sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
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
    browser.select_file(path_id(&beta_keep));
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&beta_keep).as_str())
    );

    browser.activate_folder(path_id(&alpha));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha_keep.wav"]
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn activating_collection_includes_files_with_multiple_collection_memberships() {
    let root = temp_source_root("wavecrate-gui-multi-collection-filter");
    let alpha = root.join("alpha");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    let shared = alpha.join("shared.wav");
    let other = alpha.join("other.wav");
    fs::write(&shared, []).expect("write shared sample");
    fs::write(&other, []).expect("write other sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let first = SampleCollection::new(0).expect("collection");
    let second = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&shared, first);
    browser.set_file_collection_state(&shared, second);
    browser.set_file_collection_state(&other, first);

    browser.apply_message(FolderBrowserMessage::ActivateCollection(first));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["other.wav", "shared.wav"]
    );

    browser.apply_message(FolderBrowserMessage::ActivateCollection(second));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["shared.wav"]
    );
    assert_eq!(
        browser
            .visible_collections()
            .into_iter()
            .map(|collection| (collection.collection.index(), collection.assigned_count))
            .take(2)
            .collect::<Vec<_>>(),
        vec![(0, 2), (1, 1)]
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
/// Activating a collection transfers active selection out of the folder tree.
fn activating_collection_clears_folder_selection_and_keeps_collection_as_active_source() {
    let root = temp_source_root("wavecrate-gui-collection-clears-folder");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let alpha_keep = alpha.join("alpha_keep.wav");
    let beta_keep = beta.join("beta_keep.wav");
    let beta_other = beta.join("beta_other.wav");
    fs::write(&alpha_keep, []).expect("write alpha sample");
    fs::write(&beta_keep, []).expect("write beta sample");
    fs::write(&beta_other, []).expect("write other sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&beta));
    assert_eq!(browser.selected_folder_path(), Some(beta.clone()));

    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&alpha_keep, collection);
    browser.set_file_collection_state(&beta_keep, collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(browser.selected_folder_path(), None);
    assert!(
        browser
            .visible_folders()
            .iter()
            .all(|folder| !folder.selected)
    );
    assert_eq!(
        browser
            .visible_collections()
            .into_iter()
            .find(|view| view.collection == collection)
            .map(|view| view.selected),
        Some(true)
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha_keep.wav", "beta_keep.wav"]
    );

    browser.activate_folder(path_id(&alpha));

    assert_eq!(browser.selected_folder_path(), Some(alpha.clone()));
    assert!(
        browser
            .visible_collections()
            .into_iter()
            .all(|view| !view.selected)
    );
    assert_eq!(
        browser
            .visible_folders()
            .into_iter()
            .find(|folder| folder.id == path_id(&alpha))
            .map(|folder| folder.selected),
        Some(true)
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn activating_collection_clears_stale_folder_focus_before_returning_to_tree() {
    let root = temp_source_root("wavecrate-gui-collection-clears-stale-folder");
    let alpha = root.join("alpha");
    let nested = alpha.join("nested");
    fs::create_dir_all(&nested).expect("create nested folder");
    let keep = nested.join("keep.wav");
    fs::write(&keep, []).expect("write sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let alpha_id = path_id(&alpha);

    browser.activate_folder(alpha_id.clone());
    assert!(browser.is_expanded(&alpha_id));
    assert_eq!(browser.selected_folder_path(), Some(alpha.clone()));

    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&keep, collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(browser.selection.selected_folder, path_id(&root));
    assert_eq!(browser.selected_folder_path(), None);

    browser.activate_folder(alpha_id.clone());

    assert!(browser.is_expanded(&alpha_id));
    assert_eq!(browser.selected_folder_path(), Some(alpha.clone()));
    assert_eq!(browser.selection.selected_collection, None);
    let _ = fs::remove_dir_all(root);
}
