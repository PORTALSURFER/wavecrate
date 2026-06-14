use super::*;

#[test]
fn default_gui_restores_cached_sample_indicators_from_source_scan_cache() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("restored-cache.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_id = sample_path.display().to_string();
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("source_id::gui-cache-startup"),
        source_root.path().to_path_buf(),
    );
    wavecrate::sample_sources::config::save(&crate::native_app::test_support::config::AppConfig {
        sources: vec![source.clone()],
        core: crate::native_app::test_support::config::AppSettingsCore::default(),
    })
    .expect("seed config");
    crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source])
        .save_source_scan_cache()
        .expect("persist source scan cache");

    let _waveform = crate::native_app::test_support::state::WaveformState::load_path(sample_path)
        .expect("persist waveform cache");

    let state = NativeAppState::load_default().expect("default state loads persisted cache");

    assert!(state.library.folder_browser.selected_source_loaded());
    assert!(
        !state.ui.startup.source_scan_pending,
        "cached source trees must not queue a full startup scan"
    );
    assert!(
        state.ui.startup.folder_verify_pending,
        "cached source trees should queue a bounded folder-tree refresh"
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_id),
        "startup must not probe waveform cache metadata on the UI thread"
    );
}

#[test]
fn cached_startup_queues_folder_tree_refresh_without_foreground_scan() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    fs::write(source_root.path().join("kick.wav"), [0_u8; 8]).expect("write sample");
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("source_id::gui-cache-no-startup-scan"),
        source_root.path().to_path_buf(),
    );
    wavecrate::sample_sources::config::save(&crate::native_app::test_support::config::AppConfig {
        sources: vec![source.clone()],
        core: crate::native_app::test_support::config::AppSettingsCore::default(),
    })
    .expect("seed config");
    crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source])
        .save_source_scan_cache()
        .expect("persist source scan cache");
    let mut state = NativeAppState::load_default().expect("default state loads persisted cache");
    let mut context = ui::UiUpdateContext::default();

    state.maybe_startup_source_scan(&mut context);

    assert!(
        state.library.folder_progress().is_none(),
        "cached startup must not queue a foreground source scan"
    );
    assert!(
        !state.ui.startup.source_scan_pending,
        "cached startup should not leave a full scan pending"
    );
    assert!(
        !state.ui.startup.folder_verify_pending,
        "folder-tree refresh should be consumed as a one-shot startup task"
    );
    assert!(
        state.background.folder_tree_refresh_task.active().is_some(),
        "cached startup should refresh only the folder tree in the background"
    );
    assert!(
        state.background.folder_verify_task.active().is_none(),
        "cached startup should not queue the old visible-folder verification task"
    );
}
