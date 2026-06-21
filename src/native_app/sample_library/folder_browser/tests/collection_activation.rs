use super::*;
use std::path::Path;

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
fn activating_collection_filters_audio_files_across_loaded_sources() {
    let first_root = temp_source_root("wavecrate-gui-collection-first-source");
    let second_root = temp_source_root("wavecrate-gui-collection-second-source");
    let first_keep = first_root.join("a_first_keep.wav");
    let second_keep = second_root.join("b_second_keep.wav");
    let second_other = second_root.join("c_second_other.wav");
    fs::write(&first_keep, []).expect("write first source sample");
    fs::write(&second_keep, []).expect("write second source sample");
    fs::write(&second_other, []).expect("write second source other sample");
    let sources = vec![
        wavecrate::sample_sources::SampleSource::new(first_root.clone()),
        wavecrate::sample_sources::SampleSource::new(second_root.clone()),
    ];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);

    assert!(
        browser.focus_file_across_sources(&second_keep),
        "fixture should load the second configured source"
    );

    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&first_keep, collection);
    browser.set_file_collection_state(&second_keep, collection);

    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a_first_keep.wav", "b_second_keep.wav"]
    );
    assert_eq!(
        browser
            .visible_collections()
            .into_iter()
            .find(|entry| entry.collection == collection)
            .map(|entry| entry.assigned_count),
        Some(2)
    );
    let _ = fs::remove_dir_all(first_root);
    let _ = fs::remove_dir_all(second_root);
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
fn activating_collection_includes_missing_members_from_source_db() {
    let root = temp_source_root("wavecrate-gui-missing-collection-member");
    let present = root.join("present.wav");
    fs::write(&present, []).expect("write present sample");
    let collection = SampleCollection::new(0).expect("collection");
    let db = SourceDatabase::open(&root).expect("open source db");
    seed_file_collections(&db, "missing/lost.wav", &[collection]);
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.set_file_collection_state(&present, collection);

    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    let files = browser.selected_audio_files();
    assert_eq!(
        files
            .iter()
            .map(|file| (file.name.as_str(), file.kind.as_str(), file.is_missing()))
            .collect::<Vec<_>>(),
        vec![
            ("lost.wav", "Missing", true),
            ("present.wav", "Audio", false),
        ]
    );
    assert_eq!(
        browser
            .visible_collections()
            .into_iter()
            .find(|view| view.collection == collection)
            .map(|view| view.assigned_count),
        Some(2)
    );
    assert!(
        browser
            .missing_collection_file_for_path(&root.join("missing/lost.wav"), collection)
            .is_some()
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

fn seed_file_collections(
    db: &SourceDatabase,
    relative_path: &str,
    collections: &[SampleCollection],
) {
    let path = Path::new(relative_path);
    db.upsert_file(path, 8, 1).expect("upsert source row");
    let mut batch = db.write_batch().expect("open write batch");
    for collection in collections {
        batch
            .add_collection(path, *collection)
            .expect("add collection membership");
    }
    batch.commit().expect("commit source metadata");
}
