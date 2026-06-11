use super::*;
use crate::native_app::sample_library::folder_browser::scan::verify_direct_folder;

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
fn targeted_filesystem_refresh_prunes_deleted_cached_file() {
    let root = temp_source_root("wavecrate-gui-targeted-refresh-prune-file");
    let drums = root.join("drums");
    let stale_sample = drums.join("stale.wav");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(&stale_sample, [0_u8; 8]).expect("write stale sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    fs::remove_file(&stale_sample).expect("remove stale sample");

    assert!(browser.refresh_filesystem_paths(
        "assets",
        &[std::path::PathBuf::from("drums").join("stale.wav")]
    ));

    assert!(
        browser.selected_audio_files().is_empty(),
        "targeted refresh should remove deleted samples from the cached folder listing"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn direct_folder_verify_patches_visible_folder_without_dropping_nested_cache() {
    let root = temp_source_root("wavecrate-gui-direct-folder-verify");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create nested folder");
    fs::write(kicks.join("deep.wav"), [0_u8; 8]).expect("write nested sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.find_folder(&path_id(&kicks)).is_some());
    fs::write(root.join("new-root.wav"), [1_u8; 16]).expect("write new root sample");

    let request = browser
        .selected_folder_verify_request()
        .expect("selected root should be verifiable");
    let result = verify_direct_folder(request);

    assert!(browser.apply_direct_folder_verify_result(result));
    assert!(
        browser.find_folder(&path_id(&kicks)).is_some(),
        "direct verification should preserve cached nested child folders"
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["new-root.wav"]
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

#[test]
fn selected_source_refresh_prunes_deleted_cached_files_on_finish() {
    let root = temp_source_root("wavecrate-gui-source-refresh-prune-file");
    let drums = root.join("drums");
    let stale_sample = drums.join("stale.wav");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(&stale_sample, [0_u8; 8]).expect("write stale sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["stale.wav"]
    );
    fs::remove_file(&stale_sample).expect("remove stale sample");

    let request = browser
        .begin_selected_source_scan(94)
        .expect("selected source refresh should queue");
    let result = scan_source_with_progress(request, |_| {}, |_| {});
    assert!(browser.apply_scan_finished(result));

    browser.activate_folder(path_id(&drums));
    assert!(
        browser.selected_audio_files().is_empty(),
        "refresh should remove deleted samples from the selected folder listing"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn completed_scan_discovery_prunes_deleted_root_child_before_finish() {
    let root = temp_source_root("wavecrate-gui-source-discovery-prune-root");
    let keep = root.join("keep");
    let stale = root.join("stale");
    fs::create_dir_all(&keep).expect("create keep folder");
    fs::create_dir_all(&stale).expect("create stale folder");

    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.find_folder(&path_id(&stale)).is_some());
    fs::remove_dir_all(&stale).expect("remove stale folder");

    let request = browser
        .begin_selected_source_scan(92)
        .expect("selected source refresh should queue");
    let mut discovery_events = Vec::new();
    let result = scan_source_with_progress(request, |_| {}, |event| discovery_events.push(event));

    for event in discovery_events {
        browser.apply_scan_discovered(event);
    }
    assert!(browser.find_folder(&path_id(&keep)).is_some());
    assert!(
        browser.find_folder(&path_id(&stale)).is_none(),
        "completed root discovery should replace cached root children before final scan finish"
    );

    assert!(browser.apply_scan_finished(result));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn completed_scan_discovery_prunes_deleted_nested_child_before_finish() {
    let root = temp_source_root("wavecrate-gui-source-discovery-prune-nested");
    let parent = root.join("drums");
    let keep = parent.join("keep");
    let stale = parent.join("stale");
    fs::create_dir_all(&keep).expect("create keep folder");
    fs::create_dir_all(&stale).expect("create stale folder");

    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.find_folder(&path_id(&stale)).is_some());
    fs::remove_dir_all(&stale).expect("remove stale folder");

    let request = browser
        .begin_selected_source_scan(93)
        .expect("selected source refresh should queue");
    let mut discovery_events = Vec::new();
    let result = scan_source_with_progress(request, |_| {}, |event| discovery_events.push(event));

    for event in discovery_events {
        browser.apply_scan_discovered(event);
        if browser.find_folder(&path_id(&stale)).is_none() {
            break;
        }
    }
    assert!(browser.find_folder(&path_id(&keep)).is_some());
    assert!(
        browser.find_folder(&path_id(&stale)).is_none(),
        "completed nested-folder discovery should replace stale cached children without waiting for final finish"
    );

    assert!(browser.apply_scan_finished(result));
    let _ = fs::remove_dir_all(root);
}
