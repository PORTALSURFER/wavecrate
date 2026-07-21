use crate::native_app::{
    app::SettingsMessage,
    test_support::state::{GuiMessage, NativeAppState},
};
use radiant::prelude as ui;

#[test]
fn clear_rebuildable_caches_action_removes_cache_payloads_only() {
    if std::env::var_os("WAVECRATE_CONFIG_HOME").is_some()
        || std::env::var_os("WAVECRATE_CONFIG_PROFILE").is_some()
    {
        return;
    }
    let base = tempfile::tempdir().expect("create config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = wavecrate::app_dirs::PersistenceProfileGuard::live();
    let waveform_cache = wavecrate::app_dirs::waveform_cache_dir().expect("waveform cache dir");
    let cache_payload = waveform_cache.join("cached.bin");
    std::fs::write(&cache_payload, b"cache").expect("write cache payload");
    let handoff_dir = wavecrate::app_dirs::handoff_staging_dir().expect("handoff staging dir");
    let handoff_payload = handoff_dir.join("clip.wav");
    std::fs::write(&handoff_payload, b"clip").expect("write handoff payload");
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.status.sample = String::from("ready");

    state.apply_message(
        GuiMessage::Settings(SettingsMessage::ClearRebuildableCaches),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(!cache_payload.exists());
    assert!(handoff_payload.exists());
    assert_eq!(state.audio.settings_error, None);
    assert_eq!(
        state.ui.settings.ui.global_storage_usage,
        crate::native_app::app::GlobalStorageUsageState::Loading
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Rebuildable caches cleared"),
        "{}",
        state.ui.status.sample
    );
}
