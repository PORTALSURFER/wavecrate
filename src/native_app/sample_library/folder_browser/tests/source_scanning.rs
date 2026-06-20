use super::*;
#[test]
fn source_scan_installs_finished_tree_after_placeholder_selection() {
    let root = temp_source_root("wavecrate-gui-source-scan");
    fs::create_dir_all(root.join("drums")).expect("create nested folder");
    fs::write(root.join("drums").join("kick.wav"), [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::load_default();
    let request = browser
        .begin_add_source_path(root.clone(), 42)
        .expect("new source should request scan");

    assert_eq!(browser.root_path(), root.as_path());
    assert!(browser.selected_audio_files().is_empty());

    let mut progress_events = Vec::new();
    let mut discovery_events = Vec::new();
    let result = scan_source_with_progress(
        request,
        |progress| progress_events.push(progress),
        |event| discovery_events.push(event),
    );
    assert_eq!(result.source_db_error, None);
    let db = SourceDatabase::open(&root).expect("source db");
    let entries = db.list_files().expect("source db files");
    assert!(
        entries
            .iter()
            .any(|entry| entry.relative_path == std::path::Path::new("drums/kick.wav"))
    );
    assert!(
        db.get_metadata(wavecrate::sample_sources::db::META_LAST_SCAN_COMPLETED_AT)
            .expect("scan metadata")
            .is_some()
    );
    assert!(browser.apply_scan_finished(result));

    browser.begin_select_source(root.to_string_lossy().to_string(), 43);
    browser.activate_folder(path_id(&root.join("drums")));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );
    assert!(
        progress_events
            .iter()
            .any(|progress| progress.phase == "Scanning" && progress.total == 0)
    );
    assert!(!discovery_events.is_empty());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn non_wav_audio_looking_files_are_visible_but_not_supported_audio() {
    let root = temp_source_root("wavecrate-gui-unsupported-audio");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    for name in ["kick.wav", "loop.aif", "loop.aiff", "loop.flac", "loop.mp3"] {
        fs::write(drums.join(name), [0_u8; 8]).expect("write audio-looking file");
    }

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );

    let unsupported = browser
        .selected_files()
        .iter()
        .filter(|file| file.kind == "Unsupported audio")
        .map(|file| file.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        unsupported,
        vec!["loop.aif", "loop.aiff", "loop.flac", "loop.mp3"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn apple_double_sidecars_are_hidden_from_source_browser() {
    let root = temp_source_root("wavecrate-gui-appledouble-sidecars");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
    fs::write(drums.join("._kick.wav"), [0_u8; 8]).expect("write sidecar");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    assert_eq!(
        browser
            .selected_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn source_scan_discoveries_populate_selected_tree_before_finish() {
    let root = temp_source_root("wavecrate-gui-source-streaming");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::load_default();
    let request = browser
        .begin_add_source_path(root.clone(), 77)
        .expect("new source should request scan");

    let mut progress_events = Vec::new();
    let mut discovery_events = Vec::new();
    let result = scan_source_with_progress(
        request,
        |progress| progress_events.push(progress),
        |event| discovery_events.push(event),
    );

    for event in discovery_events {
        browser.apply_scan_discovered(event);
    }
    browser.activate_folder(path_id(&drums));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );

    assert!(browser.apply_scan_finished(result));
    assert!(progress_events.iter().all(|progress| progress.total == 0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn selected_source_refresh_preserves_clicked_nested_folder() {
    let root = temp_source_root("wavecrate-gui-source-refresh-preserve-selection");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    let empty = drums.join("empty");
    fs::create_dir_all(&kicks).expect("create nested folder");
    fs::create_dir_all(&empty).expect("create empty sibling");
    fs::write(kicks.join("kick.wav"), [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let drums_id = path_id(&drums);
    let kicks_id = path_id(&kicks);
    let empty_id = path_id(&empty);

    let request = browser
        .begin_selected_source_scan(101)
        .expect("selected source refresh should queue");
    browser.activate_folder(drums_id.clone());
    browser.activate_folder(kicks_id.clone());
    let result = scan_source_with_progress(request, |_| {}, |_| {});

    assert!(browser.apply_scan_finished(result));
    let visible = browser.visible_folders();

    assert_eq!(browser.selected_folder_path(), Some(kicks.clone()));
    assert!(
        visible
            .iter()
            .any(|folder| folder.id == kicks_id && folder.selected),
        "refresh finish should keep the clicked folder visible and selected"
    );
    assert!(
        browser.tree.expanded_folders.contains(&drums_id),
        "refresh finish should keep selected-folder ancestors expanded"
    );
    assert!(
        visible
            .iter()
            .any(|folder| folder.id == empty_id && !folder.has_children),
        "empty sibling folders should remain visible without bogus disclosure state"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn source_scan_loads_deep_folders_before_click_verification() {
    let root = temp_source_root("wavecrate-gui-source-scan-deep-folders");
    let deep = root
        .join("level-1")
        .join("level-2")
        .join("level-3")
        .join("level-4")
        .join("level-5");
    fs::create_dir_all(&deep).expect("create deep folder");

    let browser = FolderBrowserState::from_root(root.clone());

    assert!(
        browser.find_folder(&path_id(&deep)).is_some(),
        "source scans should represent deep on-disk folders before a click-time refresh runs"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn source_scan_loads_all_sibling_folders_before_click_verification() {
    let root = temp_source_root("wavecrate-gui-source-scan-many-siblings");
    for index in 0..96 {
        fs::create_dir_all(root.join(format!("folder-{index:03}"))).expect("create sibling");
    }
    let late_sibling = root.join("folder-095");

    let browser = FolderBrowserState::from_root(root.clone());

    assert!(
        browser.find_folder(&path_id(&late_sibling)).is_some(),
        "source scans should not drop on-disk sibling folders beyond an arbitrary projection cap"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn batched_scan_discoveries_clone_selected_tree_once_per_batch() {
    let root = temp_source_root("wavecrate-gui-source-batch");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create nested folder");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
    fs::write(drums.join("snare.wav"), [0_u8; 8]).expect("write wav");
    let mut browser = FolderBrowserState::load_default();
    let request = browser
        .begin_add_source_path(root.clone(), 88)
        .expect("new source should request scan");

    let mut discovery_events = Vec::new();
    let result = scan_source_with_progress(request, |_| {}, |event| discovery_events.push(event));
    assert!(
        browser.apply_scan_discovered_batch(FolderScanDiscoveryBatch {
            task_id: 88,
            source_id: path_id(&root),
            events: discovery_events,
        })
    );
    browser.activate_folder(path_id(&drums));
    assert_eq!(browser.selected_audio_files().len(), 2);

    assert!(browser.apply_scan_finished(result));
    let _ = fs::remove_dir_all(root);
}
