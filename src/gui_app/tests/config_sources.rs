use super::*;

#[test]
fn default_gui_loads_persisted_sources_and_audio_output() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let source_id = wavecrate::sample_sources::SourceId::from_string("source_id::gui-test");
    wavecrate::sample_sources::config::save(&super::super::AppConfig {
        sources: vec![wavecrate::sample_sources::SampleSource::new_with_id(
            source_id,
            source_root.path().to_path_buf(),
        )],
        core: super::super::AppSettingsCore {
            audio_output: super::super::AudioOutputConfig {
                host: Some(String::from("test-host")),
                device: Some(String::from("Test Device")),
                sample_rate: Some(48_000),
                buffer_size: Some(256),
            },
            volume: 0.42,
            ..super::super::AppSettingsCore::default()
        },
    })
    .expect("seed config");

    let state = GuiAppState::load_default().expect("default state loads persisted config");

    assert_eq!(state.folder_browser.root_path(), source_root.path());
    assert!(
        state.startup_source_scan_pending,
        "uncached configured sources should scan once to build the initial tree"
    );
    assert!(!state.startup_folder_verify_pending);
    assert_eq!(state.audio_output_config.host.as_deref(), Some("test-host"));
    assert_eq!(
        state.audio_output_config.device.as_deref(),
        Some("Test Device")
    );
    assert_eq!(state.audio_output_config.sample_rate, Some(48_000));
    assert!((state.volume - 0.42).abs() < f32::EPSILON);
}

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
    wavecrate::sample_sources::config::save(&super::super::AppConfig {
        sources: vec![source.clone()],
        core: super::super::AppSettingsCore::default(),
    })
    .expect("seed config");
    super::super::FolderBrowserState::from_sample_sources(&[source])
        .save_source_scan_cache()
        .expect("persist source scan cache");

    let _waveform =
        super::super::WaveformState::load_path(sample_path).expect("persist waveform cache");

    let state = GuiAppState::load_default().expect("default state loads persisted cache");

    assert!(state.folder_browser.selected_source_loaded());
    assert!(
        !state.startup_source_scan_pending,
        "cached source trees must not queue a full startup scan"
    );
    assert!(
        state.startup_folder_verify_pending,
        "cached source trees should queue only a bounded visible-folder verification"
    );
    assert!(
        !state.cached_sample_paths.contains(&sample_id),
        "startup must not probe waveform cache metadata on the UI thread"
    );
}

#[test]
fn cached_startup_queues_visible_folder_verify_without_foreground_scan() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    fs::write(source_root.path().join("kick.wav"), [0_u8; 8]).expect("write sample");
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("source_id::gui-cache-no-startup-scan"),
        source_root.path().to_path_buf(),
    );
    wavecrate::sample_sources::config::save(&super::super::AppConfig {
        sources: vec![source.clone()],
        core: super::super::AppSettingsCore::default(),
    })
    .expect("seed config");
    super::super::FolderBrowserState::from_sample_sources(&[source])
        .save_source_scan_cache()
        .expect("persist source scan cache");
    let mut state = GuiAppState::load_default().expect("default state loads persisted cache");
    let mut context = ui::UpdateContext::default();

    state.maybe_startup_source_scan(&mut context);

    assert!(
        state.folder_progress.is_none(),
        "cached startup must not queue a foreground source scan"
    );
    assert!(
        !state.startup_source_scan_pending,
        "cached startup should not leave a full scan pending"
    );
    assert!(
        !state.startup_folder_verify_pending,
        "visible-folder verification should be consumed as a one-shot startup task"
    );
    assert!(
        state.startup_folder_verify_task.active().is_some(),
        "cached startup should verify only the visible folder in the background"
    );
}

#[test]
fn default_gui_saves_sources_and_audio_output_to_app_config() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let mut state = gui_state_for_span_tests();
    state.audio_output_config = super::super::AudioOutputConfig {
        host: Some(String::from("wasapi")),
        device: Some(String::from("Interface")),
        sample_rate: Some(96_000),
        buffer_size: None,
    };
    state.volume = 0.5;

    let request = state
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let result = super::super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result, &mut ui::UpdateContext::default());

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert_eq!(loaded.sources.len(), 1);
    assert_eq!(loaded.sources[0].root, source_root.path());
    assert_eq!(loaded.core.audio_output.host.as_deref(), Some("wasapi"));
    assert_eq!(
        loaded.core.audio_output.device.as_deref(),
        Some("Interface")
    );
    assert_eq!(loaded.core.audio_output.sample_rate, Some(96_000));
    assert!((loaded.core.volume - 0.5).abs() < f32::EPSILON);
}

#[test]
fn default_gui_removes_context_source_from_app_config() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let mut state = gui_state_for_span_tests();
    let request = state
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let result = super::super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result, &mut ui::UpdateContext::default());
    state.context_menu = Some(super::super::BrowserContextMenu {
        kind: super::super::BrowserContextTargetKind::Source,
        path: source_root.path().to_path_buf(),
        source_id: Some(source_root.path().to_string_lossy().to_string()),
        source_removable: true,
        metadata_tag: None,
        collection: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("source root"),
    });

    state.remove_context_source();

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!(loaded.sources.is_empty());
    assert!(state.sample_status.contains("Removed source"));
    assert!(state.folder_browser.root_path().ends_with("assets"));
}

#[test]
fn context_source_refresh_queues_scan_without_clearing_loaded_tree() {
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write sample");
    let mut state = gui_state_for_span_tests();
    let request = state
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let source_id = request.source_id.clone();
    let result = super::super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result, &mut ui::UpdateContext::default());
    state.context_menu = Some(super::super::BrowserContextMenu {
        kind: super::super::BrowserContextTargetKind::Source,
        path: source_root.path().to_path_buf(),
        source_id: Some(source_id.clone()),
        source_removable: true,
        metadata_tag: None,
        collection: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("source root"),
    });
    let visible_before = state.folder_browser.selected_audio_files().len();
    let mut context = ui::UpdateContext::default();

    state.refresh_context_source(&mut context);

    assert_eq!(state.context_menu, None);
    let task_id = state
        .folder_progress
        .as_ref()
        .expect("refresh should show scan progress")
        .task_id;
    assert!(
        state.folder_browser.scan_is_active(&source_id, task_id),
        "refresh should queue the next background scan task"
    );
    assert_eq!(
        state.folder_browser.selected_audio_files().len(),
        visible_before,
        "refresh should keep the current cached tree visible while the scan runs"
    );
    assert!(state.sample_status.contains("Scanning source"));
}

#[test]
fn source_filesystem_change_queues_refresh_without_clearing_loaded_tree() {
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write sample");
    let mut state = gui_state_for_span_tests();
    let request = state
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let source_id = request.source_id.clone();
    let result = super::super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result, &mut ui::UpdateContext::default());
    let visible_before = state.folder_browser.selected_audio_files().len();
    let mut context = ui::UpdateContext::default();

    state.apply_message(
        super::super::GuiMessage::SourceFilesystemChanged {
            source_id: source_id.clone(),
            paths: Vec::new(),
            overflowed: true,
        },
        &mut context,
    );

    let task_id = state
        .folder_progress
        .as_ref()
        .expect("filesystem change should show scan progress")
        .task_id;
    assert!(state.folder_browser.scan_is_active(&source_id, task_id));
    assert_eq!(
        state.folder_browser.selected_audio_files().len(),
        visible_before,
        "live sync should keep the current cached tree visible while the scan runs"
    );
}

#[test]
fn source_filesystem_change_during_scan_is_refreshed_after_scan_finishes() {
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write sample");
    let mut state = gui_state_for_span_tests();
    let request = state
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let source_id = request.source_id.clone();
    let result = super::super::folder_browser::scan_source_with_progress(request, |_| {}, |_| {});
    state.finish_folder_scan(result, &mut ui::UpdateContext::default());
    let mut context = ui::UpdateContext::default();
    state.refresh_source_after_filesystem_change(source_id.clone(), Vec::new(), true, &mut context);

    state.apply_message(
        super::super::GuiMessage::SourceFilesystemChanged {
            source_id: source_id.clone(),
            paths: Vec::new(),
            overflowed: true,
        },
        &mut context,
    );
    assert!(state.pending_source_refreshes.contains(&source_id));

    let active_task = state
        .folder_progress
        .as_ref()
        .expect("first refresh should be active")
        .task_id;
    assert!(
        state.folder_browser.scan_is_active(&source_id, active_task),
        "first scan should still own the active task"
    );
    let finished = super::super::folder_browser::scan_source_with_progress(
        super::super::folder_browser::FolderScanRequest {
            task_id: active_task,
            source_id: source_id.clone(),
            label: String::from("source"),
            root: source_root.path().to_path_buf(),
        },
        |_| {},
        |_| {},
    );
    state.finish_folder_scan(finished, &mut ui::UpdateContext::default());
    state.maybe_run_pending_source_refresh(&mut context);

    let next_task = state
        .folder_progress
        .as_ref()
        .expect("pending refresh should start after active scan")
        .task_id;
    assert_ne!(next_task, active_task);
    assert!(state.folder_browser.scan_is_active(&source_id, next_task));
}
