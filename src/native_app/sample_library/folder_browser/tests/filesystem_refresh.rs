use super::*;
use crate::native_app::sample_library::folder_browser::scan::verify_direct_folder;
use wavecrate::sample_sources::{Rating, SourceDatabase};
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
    assert!(
        browser.selected_audio_projection_cache_len_for_tests() > 0,
        "initial source load should prewarm projection cache entries"
    );
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
        browser.selected_audio_projection_cache_len_for_tests(),
        0,
        "content revision changes should invalidate stale projection cache entries"
    );

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav", "snare.wav"]
    );
    assert_eq!(
        browser.selected_audio_projection_cache_len_for_tests(),
        1,
        "projection cache should rebuild explicitly after invalidation"
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
fn selected_file_refresh_preserves_source_database_rating_metadata() {
    let root = temp_source_root("wavecrate-gui-refresh-preserves-rating");
    let drums = root.join("drums");
    let snare = drums.join("snare.wav");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(&snare, [0_u8; 8]).expect("write snare");

    let db = SourceDatabase::open(&root).expect("open source db");
    db.upsert_file(std::path::Path::new("drums/snare.wav"), 8, 1)
        .expect("upsert snare");
    db.set_tag(std::path::Path::new("drums/snare.wav"), Rating::new(2))
        .expect("set rating");
    db.set_locked(std::path::Path::new("drums/snare.wav"), true)
        .expect("set locked");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    assert!(browser.refresh_file_path(&snare));

    let refreshed = browser
        .selected_audio_files()
        .into_iter()
        .find(|file| file.name == "snare.wav")
        .expect("refreshed snare row");
    assert_eq!(refreshed.rating, Rating::new(2));
    assert!(refreshed.rating_locked);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn direct_folder_verify_preserves_rating_after_external_rename() {
    let root = temp_source_root("wavecrate-gui-verify-external-rename-rating");
    let drums = root.join("drums");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(&kick, [0_u8; 8]).expect("write kick");

    let db = SourceDatabase::open(&root).expect("open source db");
    wavecrate::sample_sources::scanner::scan_once(&db).expect("initial scan");
    db.set_tag(std::path::Path::new("drums/kick.wav"), Rating::new(2))
        .expect("set rating");
    db.set_locked(std::path::Path::new("drums/kick.wav"), true)
        .expect("set locked");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    let request = browser
        .selected_folder_verify_request()
        .expect("selected folder should be verifiable");
    fs::rename(&kick, &snare).expect("external rename");

    let result = verify_direct_folder(request);
    assert!(browser.apply_direct_folder_verify_result(result));

    let renamed = browser
        .selected_audio_files()
        .into_iter()
        .find(|file| file.name == "snare.wav")
        .expect("renamed snare row");
    assert_eq!(renamed.rating, Rating::new(2));
    assert!(renamed.rating_locked);
    assert_eq!(
        db.tag_for_path(std::path::Path::new("drums/snare.wav"))
            .expect("read renamed rating"),
        Some(Rating::new(2))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn selected_source_refresh_preserves_rating_after_external_move() {
    let root = temp_source_root("wavecrate-gui-refresh-external-move-rating");
    let drums = root.join("drums");
    let loops = root.join("loops");
    let kick = drums.join("kick.wav");
    let moved = loops.join("kick.wav");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    fs::write(&kick, [0_u8; 8]).expect("write kick");

    let db = SourceDatabase::open(&root).expect("open source db");
    wavecrate::sample_sources::scanner::scan_once(&db).expect("initial scan");
    db.set_tag(std::path::Path::new("drums/kick.wav"), Rating::new(2))
        .expect("set rating");
    db.set_locked(std::path::Path::new("drums/kick.wav"), true)
        .expect("set locked");

    let mut browser = FolderBrowserState::from_root(root.clone());
    fs::rename(&kick, &moved).expect("external move");
    let request = browser
        .begin_selected_source_scan(95)
        .expect("selected source refresh should queue");
    let result = scan_source_with_progress(request, |_| {}, |_| {});
    assert!(browser.apply_scan_finished(result));

    browser.activate_folder(path_id(&loops));
    let moved_row = browser
        .selected_audio_files()
        .into_iter()
        .find(|file| file.name == "kick.wav")
        .expect("moved kick row");
    assert_eq!(moved_row.rating, Rating::new(2));
    assert!(moved_row.rating_locked);
    assert_eq!(
        db.tag_for_path(std::path::Path::new("loops/kick.wav"))
            .expect("read moved rating"),
        Some(Rating::new(2))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn direct_folder_verify_keeps_unchanged_selected_folder_visible() {
    let root = temp_source_root("wavecrate-gui-direct-folder-verify-unchanged");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create nested folder");
    fs::write(kicks.join("deep.wav"), [0_u8; 8]).expect("write nested sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    let request = browser
        .selected_folder_verify_request()
        .expect("selected child folder should be verifiable");
    let result = verify_direct_folder(request);

    assert!(
        !browser.apply_direct_folder_verify_result(result),
        "unchanged folder verification should not mutate the tree"
    );
    assert!(
        browser.find_folder(&path_id(&drums)).is_some(),
        "unchanged selected folder should remain visible after click-triggered verification"
    );
    assert!(
        browser.find_folder(&path_id(&kicks)).is_some(),
        "unchanged selected folder should keep its cached child folders"
    );
    assert_eq!(browser.selected_folder_path(), Some(drums.clone()));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn direct_folder_verify_prunes_deleted_visible_file() {
    let root = temp_source_root("wavecrate-gui-direct-folder-verify-prune-file");
    let drums = root.join("drums");
    let stale_sample = drums.join("stale.wav");
    let keep_sample = drums.join("keep.wav");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(&stale_sample, [0_u8; 8]).expect("write stale sample");
    fs::write(&keep_sample, [1_u8; 8]).expect("write keep sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&stale_sample));
    let request = browser
        .selected_folder_verify_request()
        .expect("selected folder should be verifiable");
    fs::remove_file(&stale_sample).expect("remove stale sample");

    let result = verify_direct_folder(request);
    assert!(browser.apply_direct_folder_verify_result(result));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["keep.wav"]
    );
    assert_eq!(
        browser.selected_file_paths(),
        Vec::<std::path::PathBuf>::new(),
        "selection should drop a file pruned by folder verification"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn direct_folder_verify_prunes_deleted_selected_folder() {
    let root = temp_source_root("wavecrate-gui-direct-folder-verify-prune-folder");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(drums.join("stale.wav"), [0_u8; 8]).expect("write stale sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    let request = browser
        .selected_folder_verify_request()
        .expect("selected folder should be verifiable");
    fs::remove_dir_all(&drums).expect("remove selected folder");

    let result = verify_direct_folder(request);
    assert!(browser.apply_direct_folder_verify_result(result));

    assert!(browser.find_folder(&path_id(&drums)).is_none());
    assert_eq!(browser.selected_folder_path(), Some(root.clone()));
    assert!(browser.selected_audio_files().is_empty());
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
fn folder_tree_refresh_prunes_deleted_folders_and_preserves_files() {
    let root = temp_source_root("wavecrate-gui-folder-tree-refresh-prune");
    let stale = root.join("stale");
    let keep = root.join("keep");
    let keep_file = keep.join("keep.wav");
    fs::create_dir_all(&stale).expect("create stale folder");
    fs::create_dir_all(&keep).expect("create keep folder");
    fs::write(&keep_file, [0_u8; 8]).expect("write keep sample");

    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.find_folder(&path_id(&stale)).is_some());
    assert!(browser.find_folder(&path_id(&keep)).is_some());
    fs::remove_dir_all(&stale).expect("remove stale folder");

    let result = refresh_folder_tree_only(FolderTreeRefreshRequest {
        source_id: String::from("assets"),
        label: String::from("Assets"),
        root: root.clone(),
    });
    assert!(browser.apply_folder_tree_refresh_result(result));

    assert!(browser.find_folder(&path_id(&stale)).is_none());
    browser.activate_folder(path_id(&keep));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["keep.wav"],
        "folder-only refresh should preserve cached file rows for folders that still exist"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_tree_refresh_adds_new_empty_folder() {
    let root = temp_source_root("wavecrate-gui-folder-tree-refresh-add-empty");
    let added = root.join("added-empty");

    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.find_folder(&path_id(&added)).is_none());
    fs::create_dir_all(&added).expect("create added folder");

    let result = refresh_folder_tree_only(FolderTreeRefreshRequest {
        source_id: String::from("assets"),
        label: String::from("Assets"),
        root: root.clone(),
    });
    assert!(browser.apply_folder_tree_refresh_result(result));

    let folder = browser
        .find_folder(&path_id(&added))
        .expect("new empty folder should be visible");
    assert!(
        !folder.contains_audio(),
        "new folder should be marked empty by the existing contains-audio styling contract"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_tree_refresh_reconciles_deleted_selected_folder() {
    let root = temp_source_root("wavecrate-gui-folder-tree-refresh-selected");
    let stale = root.join("stale");
    fs::create_dir_all(&stale).expect("create stale folder");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&stale));
    fs::remove_dir_all(&stale).expect("remove stale folder");

    let result = refresh_folder_tree_only(FolderTreeRefreshRequest {
        source_id: String::from("assets"),
        label: String::from("Assets"),
        root: root.clone(),
    });
    assert!(browser.apply_folder_tree_refresh_result(result));

    assert!(browser.find_folder(&path_id(&stale)).is_none());
    assert_eq!(browser.selected_folder_path(), Some(root.clone()));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_tree_refresh_ignores_stale_source_result() {
    let first_root = temp_source_root("wavecrate-gui-folder-tree-refresh-first");
    let second_root = temp_source_root("wavecrate-gui-folder-tree-refresh-second");
    let stale_first = first_root.join("stale-first");
    let second_child = second_root.join("second-child");
    fs::create_dir_all(&stale_first).expect("create first child");
    fs::create_dir_all(&second_child).expect("create second child");
    let first_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("first"),
        first_root.clone(),
    );
    let second_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("second"),
        second_root.clone(),
    );
    let mut browser =
        FolderBrowserState::from_sample_sources(&[first_source.clone(), second_source.clone()]);
    let second_scan = browser
        .begin_source_scan(String::from("second"), 42)
        .expect("second source scan can queue");
    let second_result = scan_source_with_progress(second_scan, |_| {}, |_| {});
    assert!(browser.apply_scan_finished(second_result));
    assert!(
        browser
            .begin_select_source(String::from("second"), 43)
            .is_none()
    );
    fs::remove_dir_all(&stale_first).expect("remove first child");

    let stale_result = refresh_folder_tree_only(FolderTreeRefreshRequest {
        source_id: String::from("first"),
        label: String::from("First"),
        root: first_root.clone(),
    });
    assert!(!browser.apply_folder_tree_refresh_result(stale_result));

    assert_eq!(browser.selected_source_id(), "second");
    assert!(browser.find_folder(&path_id(&second_child)).is_some());
    let _ = fs::remove_dir_all(first_root);
    let _ = fs::remove_dir_all(second_root);
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
