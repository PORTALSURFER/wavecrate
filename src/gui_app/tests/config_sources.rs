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
        state.cached_sample_paths.contains(&sample_id),
        "startup should mark cached rows bright when source contents are restored from scan cache"
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
    state.finish_folder_scan(result);

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
    state.finish_folder_scan(result);
    state.context_menu = Some(super::super::BrowserContextMenu {
        kind: super::super::BrowserContextTargetKind::Source,
        path: source_root.path().to_path_buf(),
        source_id: Some(source_root.path().to_string_lossy().to_string()),
        metadata_tag: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("source root"),
    });

    state.remove_context_source();

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!(loaded.sources.is_empty());
    assert!(state.sample_status.contains("Removed source"));
    assert!(state.folder_browser.root_path().ends_with("assets"));
}
