use super::*;

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
fn removing_last_user_source_restores_default_assets_source() {
    let root = temp_source_root("wavecrate-gui-remove-last-source");
    let sources = vec![wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string(root.to_string_lossy().to_string()),
        root.clone(),
    )];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);

    browser
        .remove_source(root.to_string_lossy().as_ref())
        .expect("last user source should be removable");

    assert!(browser.root_path().ends_with("assets"));
    assert!(browser.configured_sample_sources().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn default_assets_source_is_not_removable() {
    let mut browser = FolderBrowserState::load_default();

    let error = browser
        .remove_source("assets")
        .expect_err("default assets source should stay registered");

    assert_eq!(error, "Default source cannot be removed");
    assert_eq!(browser.source_labels(), vec![String::from("Assets")]);
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
    assert!(
        reloaded.begin_selected_source_scan(7).is_none(),
        "cached selected source should not queue a startup scan"
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
    let _ = fs::remove_dir_all(root);
}
