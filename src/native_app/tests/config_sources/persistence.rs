use super::*;

#[test]
fn default_gui_loads_persisted_sources_and_audio_output() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let source_id = wavecrate::sample_sources::SourceId::from_string("source_id::gui-test");
    wavecrate::sample_sources::config::save(&crate::native_app::test_support::config::AppConfig {
        sources: vec![wavecrate::sample_sources::SampleSource::new_with_id(
            source_id,
            source_root.path().to_path_buf(),
        )],
        core: crate::native_app::test_support::config::AppSettingsCore {
            audio_output: crate::native_app::test_support::audio::AudioOutputConfig {
                host: Some(String::from("test-host")),
                device: Some(String::from("Test Device")),
                sample_rate: Some(48_000),
                buffer_size: Some(256),
            },
            volume: 0.42,
            ..crate::native_app::test_support::config::AppSettingsCore::default()
        },
    })
    .expect("seed config");

    let state = NativeAppState::load_default().expect("default state loads persisted config");

    assert_eq!(state.library.folder_browser.root_path(), source_root.path());
    assert!(
        state.ui.startup.source_scan_pending,
        "uncached configured sources should scan once to build the initial tree"
    );
    assert!(!state.ui.startup.folder_verify_pending);
    assert_eq!(state.audio.output_config.host.as_deref(), Some("test-host"));
    assert_eq!(
        state.audio.output_config.device.as_deref(),
        Some("Test Device")
    );
    assert_eq!(state.audio.output_config.sample_rate, Some(48_000));
    assert!((state.audio.volume - 0.42).abs() < f32::EPSILON);
}

#[test]
fn default_gui_saves_sources_and_audio_output_to_app_config() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let mut state = gui_state_for_span_tests();
    state.audio.output_config = crate::native_app::test_support::audio::AudioOutputConfig {
        host: Some(String::from("wasapi")),
        device: Some(String::from("Interface")),
        sample_rate: Some(96_000),
        buffer_size: None,
    };
    state.audio.volume = 0.5;

    let request = state
        .library
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        request,
        |_| {},
        |_| {},
    );
    state.finish_folder_scan(result, &mut ui::UiUpdateContext::default());

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
        .library
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        request,
        |_| {},
        |_| {},
    );
    state.finish_folder_scan(result, &mut ui::UiUpdateContext::default());
    state.ui.browser_interaction.context_menu = Some(
        crate::native_app::test_support::context_menu::BrowserContextMenu {
            kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Source,
            path: source_root.path().to_path_buf(),
            source_id: Some(source_root.path().to_string_lossy().to_string()),
            source_removable: true,
            folder_locked: false,
            folder_lock_inherited: false,
            metadata_tag: None,
            collection: None,
            sample_missing: false,
            anchor: Point::new(12.0, 24.0),
            title: String::from("source root"),
        },
    );

    state.remove_context_source();

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!(loaded.sources.is_empty());
    assert!(state.ui.status.sample.contains("Removed source"));
    assert!(state.library.folder_browser.root_path().ends_with("assets"));
}
