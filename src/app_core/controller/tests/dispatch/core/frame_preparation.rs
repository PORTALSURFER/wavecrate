use super::*;
use crate::app::controller::{
    startup_audio_refresh_count_for_tests, with_stubbed_startup_audio_refresh_for_tests,
};
use crate::app_core::controller::UiFramePreparationPlan;

#[test]
fn prepare_ui_frame_animation_only_updates_fps_when_not_playing() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    assert!(controller.average_fps().is_none());
    controller.prepare_ui_frame(true);
    assert!(controller.average_fps().is_none());

    sleep(Duration::from_millis(2));
    controller.prepare_ui_frame(true);

    assert!(controller.average_fps().is_some());
}

#[test]
fn browser_retained_pull_plan_skips_startup_lanes() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller
            .apply_configuration(crate::sample_sources::config::AppConfig::default())
            .expect("apply startup config");

        controller.prepare_ui_frame_with_plan(UiFramePreparationPlan::BrowserRetainedPull);
        controller.prepare_ui_frame_with_plan(UiFramePreparationPlan::BrowserRetainedPull);

        assert!(controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 0);
    });
}

#[test]
fn full_pull_plan_runs_startup_lanes() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller
            .apply_configuration(crate::sample_sources::config::AppConfig::default())
            .expect("apply startup config");

        controller.prepare_ui_frame_with_plan(UiFramePreparationPlan::Full);
        controller.prepare_ui_frame_with_plan(UiFramePreparationPlan::Full);

        assert!(!controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 1);
    });
}

#[test]
fn startup_retained_pull_plan_runs_startup_lanes() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller
            .apply_configuration(crate::sample_sources::config::AppConfig::default())
            .expect("apply startup config");

        controller.prepare_ui_frame_with_plan(UiFramePreparationPlan::StartupRetainedPull);
        controller.prepare_ui_frame_with_plan(UiFramePreparationPlan::StartupRetainedPull);

        assert!(!controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 1);
    });
}
