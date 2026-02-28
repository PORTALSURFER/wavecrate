use super::*;
use crate::waveform::{WaveformChannelView, WaveformRenderer};

#[test]
fn normalized_audition_setter_syncs_ui_when_settings_already_match() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.settings.controls.normalized_audition_enabled = true;
    controller.ui.waveform.normalized_audition_enabled = false;

    controller.set_normalized_audition_enabled(true);

    assert!(controller.settings.controls.normalized_audition_enabled);
    assert!(controller.ui.waveform.normalized_audition_enabled);
}

#[test]
fn channel_view_setter_syncs_ui_when_settings_already_match() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.settings.controls.waveform_channel_view = WaveformChannelView::SplitStereo;
    controller.ui.waveform.channel_view = WaveformChannelView::Mono;
    controller.ui.controls.waveform_channel_view = WaveformChannelView::Mono;

    controller.set_waveform_channel_view(WaveformChannelView::SplitStereo);

    assert_eq!(
        controller.settings.controls.waveform_channel_view,
        WaveformChannelView::SplitStereo
    );
    assert_eq!(
        controller.ui.waveform.channel_view,
        WaveformChannelView::SplitStereo
    );
    assert_eq!(
        controller.ui.controls.waveform_channel_view,
        WaveformChannelView::SplitStereo
    );
}

#[test]
fn transient_markers_setter_syncs_ui_when_settings_already_match() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.settings.controls.transient_markers_enabled = true;
    controller.settings.controls.transient_snap_enabled = true;
    controller.ui.waveform.transient_markers_enabled = false;
    controller.ui.waveform.transient_snap_enabled = false;

    controller.set_transient_markers_enabled(true);

    assert!(controller.settings.controls.transient_markers_enabled);
    assert!(controller.ui.waveform.transient_markers_enabled);
    assert!(controller.ui.waveform.transient_snap_enabled);
}

#[test]
fn wheel_zoom_speed_mapping_is_monotonic() {
    let slow = wheel_zoom_speed_to_factor(0.2);
    let medium = wheel_zoom_speed_to_factor(1.0);
    let fast = wheel_zoom_speed_to_factor(10.0);

    assert!(slow > medium, "expected slower speed to zoom less per step");
    assert!(medium > fast, "expected higher speed to zoom more per step");
}

#[test]
fn wheel_zoom_speed_round_trips_with_factor() {
    let speeds = [0.2, 0.5, 1.0, 2.0, 8.0, 16.0];
    for speed in speeds {
        let factor = wheel_zoom_speed_to_factor(speed);
        let round_tripped = wheel_zoom_factor_to_speed(factor);
        assert!(
            (speed - round_tripped).abs() < 0.02,
            "speed {speed} round-tripped to {round_tripped} via factor {factor}"
        );
    }
}
