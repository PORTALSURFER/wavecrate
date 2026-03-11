use super::*;
use crate::app_core::app_api::state::SampleBrowserIndex;

#[test]
fn status_bar_right_text_shows_column() {
    assert_eq!(status_bar_right_text(0), "col: 1/3");
}

/// Status-bar column text should remain stable for repeated equivalent inputs.
#[test]
fn status_bar_right_text_is_stable_across_input() {
    assert_eq!(status_bar_right_text(2), "col: 3/3");
}

#[test]
/// Motion projection should derive right-status text directly from selected column.
fn motion_projection_sets_status_right_from_selected_column() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.status.text = "x".repeat(8_192);
    controller.ui.browser.selected = Some(SampleBrowserIndex {
        column: TriageFlagColumn::Keep,
        row: 0,
    });

    let motion = project_motion_model(&mut controller);

    assert_eq!(motion.status_right, "col: 3/3");
}

#[test]
/// Motion projection should preserve micro-precision playhead and waveform view bounds.
fn motion_projection_preserves_waveform_micro_precision() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.500_456;
    controller.ui.waveform.view.start = 0.500_123;
    controller.ui.waveform.view.end = 0.501_123;

    let motion = project_motion_model(&mut controller);

    assert_eq!(motion.waveform_playhead_micros, Some(500_456));
    assert_eq!(motion.waveform_view_start_micros, 500_123);
    assert_eq!(motion.waveform_view_end_micros, 501_123);
}
