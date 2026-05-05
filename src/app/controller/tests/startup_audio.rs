use super::super::{
    AppController, WaveformRenderer, startup_audio_refresh_count_for_tests,
    with_stubbed_startup_audio_refresh_for_tests,
};
use crate::app_core::controller::AppControllerNativeRuntimeExt;

#[test]
fn apply_configuration_arms_deferred_startup_audio_refresh() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let mut cfg = crate::sample_sources::config::AppConfig::default();
        cfg.core.audio_output.host = Some("wasapi".into());
        cfg.core.audio_output.device = Some("Studio".into());
        cfg.core.audio_output.sample_rate = Some(48_000);
        cfg.core.audio_input.host = Some("asio".into());
        cfg.core.audio_input.device = Some("Mic".into());
        cfg.core.audio_input.sample_rate = Some(44_100);
        cfg.core.audio_input.channels = vec![1, 2];

        controller
            .apply_configuration(cfg.clone())
            .expect("apply startup config");

        assert_eq!(controller.settings.audio_output, cfg.core.audio_output);
        assert_eq!(controller.ui.audio.selected, cfg.core.audio_output);
        assert_eq!(controller.settings.audio_input, cfg.core.audio_input);
        assert_eq!(controller.ui.audio.input_selected, cfg.core.audio_input);
        assert!(controller.ui.audio.hosts.is_empty());
        assert!(controller.ui.audio.devices.is_empty());
        assert!(controller.ui.audio.sample_rates.is_empty());
        assert!(controller.ui.audio.input_hosts.is_empty());
        assert!(controller.ui.audio.input_devices.is_empty());
        assert!(controller.ui.audio.input_sample_rates.is_empty());
        assert_eq!(controller.ui.audio.input_channel_count, 0);
        assert!(controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 0);
    });
}

#[test]
fn startup_audio_refresh_waits_until_after_first_prepare() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

        controller
            .apply_configuration(crate::sample_sources::config::AppConfig::default())
            .expect("apply startup config");

        controller.prepare_native_frame(false);
        assert!(controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 0);

        controller.prepare_native_frame(false);
        assert!(!controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 1);
    });
}
