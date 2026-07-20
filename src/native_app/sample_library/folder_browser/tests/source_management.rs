use super::*;

fn deferred_source(
    id: &str,
    role: wavecrate::sample_sources::SourceRole,
) -> wavecrate::sample_sources::SampleSource {
    let mut source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(id),
        PathBuf::from(format!("test-sources/{id}")),
    );
    source.role = role;
    source
}

#[test]
fn source_reorder_drag_commits_stable_source_order_without_changing_selection_or_roles() {
    let sources = vec![
        deferred_source("source-a", wavecrate::sample_sources::SourceRole::Primary),
        deferred_source("source-b", wavecrate::sample_sources::SourceRole::Normal),
        deferred_source("source-c", wavecrate::sample_sources::SourceRole::Protected),
    ];
    let mut browser = FolderBrowserState::from_sample_sources_deferred(&sources);

    assert!(!browser.apply_source_reorder_drag(
        String::from("source-a"),
        radiant::widgets::DragHandleMessage::started(radiant::prelude::Point::new(20.0, 100.0)),
    ));
    assert!(!browser.apply_source_reorder_drag(
        String::from("source-a"),
        radiant::widgets::DragHandleMessage::moved(radiant::prelude::Point::new(20.0, 148.0)),
    ));
    assert_eq!(browser.source_reorder_drag_source_id(), Some("source-a"));
    assert_eq!(browser.source_reorder_target_source_id(), Some("source-c"));
    assert_eq!(
        browser.source_reorder_drop_marker_after("source-c"),
        Some(true)
    );
    assert_eq!(browser.source_reorder_drop_marker_after("source-b"), None);
    assert_eq!(
        browser
            .configured_sample_sources()
            .into_iter()
            .map(|source| source.id.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec!["source-a", "source-b", "source-c"],
        "moving the pointer should preview without mutating source order"
    );

    assert!(browser.apply_source_reorder_drag(
        String::from("source-a"),
        radiant::widgets::DragHandleMessage::ended(radiant::prelude::Point::new(20.0, 148.0)),
    ));

    let reordered = browser.configured_sample_sources();
    assert_eq!(
        reordered
            .iter()
            .map(|source| source.id.as_str())
            .collect::<Vec<_>>(),
        vec!["source-b", "source-c", "source-a"]
    );
    assert_eq!(
        reordered
            .iter()
            .map(|source| source.role)
            .collect::<Vec<_>>(),
        vec![
            wavecrate::sample_sources::SourceRole::Normal,
            wavecrate::sample_sources::SourceRole::Protected,
            wavecrate::sample_sources::SourceRole::Primary,
        ]
    );
    assert_eq!(browser.selected_source_id(), "source-a");
    assert!(!browser.source_reorder_drag_active());
}

#[test]
fn cancelling_source_reorder_keeps_original_order() {
    let sources = vec![
        deferred_source("source-a", wavecrate::sample_sources::SourceRole::Normal),
        deferred_source("source-b", wavecrate::sample_sources::SourceRole::Normal),
    ];
    let mut browser = FolderBrowserState::from_sample_sources_deferred(&sources);

    browser.apply_source_reorder_drag(
        String::from("source-a"),
        radiant::widgets::DragHandleMessage::started(radiant::prelude::Point::new(20.0, 100.0)),
    );
    browser.apply_source_reorder_drag(
        String::from("source-a"),
        radiant::widgets::DragHandleMessage::moved(radiant::prelude::Point::new(20.0, 124.0)),
    );
    assert!(!browser.apply_source_reorder_drag(
        String::from("source-a"),
        radiant::widgets::DragHandleMessage::cancelled(radiant::prelude::Point::new(20.0, 124.0)),
    ));

    assert_eq!(
        browser
            .configured_sample_sources()
            .into_iter()
            .map(|source| source.id.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec!["source-a", "source-b"]
    );
    assert!(!browser.source_reorder_drag_active());
}

#[test]
fn removing_selected_user_source_falls_back_to_next_source() {
    let first = temp_source_root("wavecrate-gui-remove-source-first");
    let second = temp_source_root("wavecrate-gui-remove-source-second");
    let sources = vec![
        wavecrate::sample_sources::SampleSource::new_with_id(
            wavecrate::sample_sources::SourceId::from_string(first.to_string_lossy().to_string()),
            first.clone(),
        ),
        wavecrate::sample_sources::SampleSource::new_with_id(
            wavecrate::sample_sources::SourceId::from_string(second.to_string_lossy().to_string()),
            second.clone(),
        ),
    ];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);

    let removed = browser
        .remove_source(first.to_string_lossy().as_ref())
        .expect("selected user source should be removable");

    assert_eq!(removed.root, first);
    assert_eq!(browser.root_path(), second.as_path());
    assert_eq!(
        browser
            .configured_sample_sources()
            .into_iter()
            .map(|source| source.root)
            .collect::<Vec<_>>(),
        vec![second.clone()]
    );
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[test]
fn removing_last_user_source_leaves_source_list_empty() {
    let root = temp_source_root("wavecrate-gui-remove-last-source");
    let sources = vec![wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(root.to_string_lossy().to_string()),
        root.clone(),
    )];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);

    browser
        .remove_source(root.to_string_lossy().as_ref())
        .expect("last user source should be removable");

    assert_eq!(browser.source_labels(), Vec::<String>::new());
    assert_eq!(browser.selected_source_id(), "");
    assert!(browser.selected_files().is_empty());
    assert!(browser.selected_audio_files().is_empty());
    assert!(browser.configured_sample_sources().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn default_folder_browser_has_no_sources() {
    let browser = FolderBrowserState::load_default();

    assert_eq!(browser.source_labels(), Vec::<String>::new());
    assert_eq!(browser.selected_source_id(), "");
    assert!(browser.selected_files().is_empty());
    assert!(browser.selected_audio_files().is_empty());
    assert!(browser.configured_sample_sources().is_empty());
}

#[test]
fn legacy_default_assets_source_is_not_shown() {
    let assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let sources = vec![wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(String::from("assets")),
        assets_root,
    )];
    let browser = FolderBrowserState::from_sample_sources_deferred(&sources);

    assert_eq!(browser.source_labels(), Vec::<String>::new());
    assert_eq!(browser.selected_source_id(), "");
    assert!(browser.selected_files().is_empty());
    assert!(browser.selected_audio_files().is_empty());
    assert!(browser.configured_sample_sources().is_empty());
}

#[test]
fn deferred_sample_sources_start_with_placeholder_and_queue_selected_scan() {
    let root = temp_source_root("wavecrate-gui-deferred-source");
    let sources = vec![wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(root.to_string_lossy().to_string()),
        root.clone(),
    )];
    let mut browser = FolderBrowserState::from_sample_sources_deferred(&sources);

    assert!(!browser.selected_source_loaded());
    let request = browser
        .begin_selected_source_scan(7)
        .expect("selected source scan should be queued");

    assert_eq!(request.task_id, 7);
    assert_eq!(request.root, root);
    assert!(browser.scan_is_active(&request.source_id, 7));
    let _ = fs::remove_dir_all(request.root);
}

#[test]
fn deferred_sample_sources_reuse_persisted_scan_cache() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let root = temp_source_root("wavecrate-gui-deferred-source-cache");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
    let sources = vec![wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(root.to_string_lossy().to_string()),
        root.clone(),
    )];
    let browser = FolderBrowserState::from_sample_sources(&sources);
    browser
        .save_source_scan_cache()
        .expect("persist source scan cache");

    let mut reloaded = FolderBrowserState::from_sample_sources_deferred(&sources);

    assert!(reloaded.selected_source_loaded());
    let request = reloaded
        .begin_selected_source_scan(7)
        .expect("cached selected source should still queue a refresh scan");
    assert_eq!(request.root, root);
    assert!(reloaded.scan_is_active(&request.source_id, 7));
    reloaded.activate_folder(path_id(&drums));
    assert_eq!(
        reloaded
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn selecting_cached_source_during_background_rescan_keeps_tree_loaded() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let first_root = temp_source_root("wavecrate-gui-cached-rescan-first");
    let second_root = temp_source_root("wavecrate-gui-cached-rescan-second");
    let second_sample = second_root.join("cached.wav");
    fs::write(&second_sample, [0_u8; 8]).expect("write cached sample");
    let sources = [
        wavecrate::sample_sources::SampleSource::new_with_id(
            wavecrate::sample_sources::SourceId::from_string(
                first_root.to_string_lossy().to_string(),
            ),
            first_root.clone(),
        ),
        wavecrate::sample_sources::SampleSource::new_with_id(
            wavecrate::sample_sources::SourceId::from_string(
                second_root.to_string_lossy().to_string(),
            ),
            second_root.clone(),
        ),
    ];
    let second_id = sources[1].id.as_str().to_string();
    let mut seeded = FolderBrowserState::from_sample_sources(&sources);
    let request = seeded
        .begin_select_source(second_id.clone(), 8)
        .expect("second source scan");
    assert!(seeded.apply_scan_finished(scan_source_with_progress(request, |_| {}, |_| {})));
    seeded
        .save_source_scan_cache()
        .expect("persist both source trees");

    let mut reloaded = FolderBrowserState::from_sample_sources_deferred(&sources);
    reloaded
        .begin_source_scan(second_id.clone(), 9)
        .expect("cached background rescan");
    assert!(reloaded.select_source_without_scan(second_id.clone()));

    assert_eq!(reloaded.selected_source_id(), second_id);
    assert!(reloaded.selected_source_loaded());
    assert_eq!(
        reloaded
            .selected_audio_files()
            .iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>(),
        vec![second_sample.to_string_lossy().to_string()]
    );
    let _ = fs::remove_dir_all(first_root);
    let _ = fs::remove_dir_all(second_root);
}

#[test]
fn deferred_missing_source_keeps_cached_tree_and_blocks_refresh() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let root = temp_source_root("wavecrate-gui-deferred-missing-source-cache");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
    let source_id = root.to_string_lossy().to_string();
    let sources = vec![wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(source_id.clone()),
        root.clone(),
    )];
    let browser = FolderBrowserState::from_sample_sources(&sources);
    browser
        .save_source_scan_cache()
        .expect("persist source scan cache");
    fs::remove_dir_all(&root).expect("remove source root");

    let mut reloaded = FolderBrowserState::from_sample_sources_deferred(&sources);

    assert!(reloaded.source_is_missing(&source_id));
    assert!(reloaded.selected_source_loaded());
    assert!(reloaded.begin_selected_source_scan(7).is_none());
    assert!(
        reloaded
            .selected_source_folder_tree_refresh_request()
            .is_none()
    );
    reloaded.activate_folder(path_id(&drums));
    assert_eq!(
        reloaded
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );
}
