use super::*;

#[test]
fn volume_slider_drag_emits_normalized_volume() {
    assert_eq!(
        crate::native_app::test_support::settings::volume_slider(0.25).view_dispatch_widget_output(
            crate::native_app::test_support::settings::VOLUME_SLIDER_ID,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::SliderMessage::ValueChanged {
                value: 0.75
            },),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::Settings(
                crate::native_app::app::SettingsMessage::SetVolume(0.75)
            )
        )
    );
}

#[test]
fn default_gui_volume_state_clamps() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.set_volume(1.5);
    assert_eq!(state.audio.volume, 1.0);

    state.set_volume(-0.5);
    assert_eq!(state.audio.volume, 0.0);
}

#[test]
fn default_gui_volume_drag_defers_config_persistence_until_debounce() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let mut state = super::gui_state_for_span_tests();
    state.persist_user_configuration("test.seed", Instant::now());

    state.set_volume(0.25);

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!(
        (loaded.core.volume - crate::native_app::test_support::state::DEFAULT_VOLUME).abs()
            < f32::EPSILON
    );
    assert!(state.audio.volume_persist_deadline.is_some());

    state.audio.volume_persist_deadline = Some(Instant::now() - Duration::from_millis(1));
    state.advance_frame();

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!((loaded.core.volume - 0.25).abs() < f32::EPSILON);
    assert!(state.audio.volume_persist_deadline.is_none());
}
