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

#[test]
fn selected_folder_audio_projection_refreshes_after_file_update() {
    let root = temp_source_root("wavecrate-gui-folder-projection-refresh");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&kick, [0_u8; 8]).expect("write kick");

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

    fs::write(&snare, [0_u8; 8]).expect("write snare");
    assert!(browser.refresh_file_path(&snare));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav", "snare.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn selected_source_refresh_prunes_deleted_cached_folders_on_finish() {
    let root = temp_source_root("wavecrate-gui-source-refresh-prune");
    let stale = root.join("stale");
    fs::create_dir_all(&stale).expect("create stale folder");
    fs::write(stale.join("old.wav"), [0_u8; 8]).expect("write old sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.find_folder(&path_id(&stale)).is_some());
    fs::remove_dir_all(&stale).expect("remove stale folder");

    let request = browser
        .begin_selected_source_scan(91)
        .expect("selected source refresh should queue");
    let result = scan_source_with_progress(request, |_| {}, |_| {});
    assert!(browser.apply_scan_finished(result));

    assert!(browser.find_folder(&path_id(&stale)).is_none());
    let _ = fs::remove_dir_all(root);
}
