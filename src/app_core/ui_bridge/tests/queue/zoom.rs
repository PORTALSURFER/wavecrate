use super::*;

/// Zoom-to-selection and zoom-full should override discrete zoom deltas.
#[test]
fn waveform_action_queue_zoom_overrides_delta() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 3,
            anchor_ratio_micros: Some(250_000),
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveformToSelection
    )));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert_eq!(queue.zoom_anchor_ratio_micros, None);
    assert!(queue.zoom_to_selection);
    assert!(!queue.zoom_full);

    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull
    )));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert!(!queue.zoom_to_selection);
    assert!(queue.zoom_full);
}

/// Discrete zoom coalescing should keep the most recent pointer anchor.
#[test]
fn waveform_action_queue_keeps_latest_zoom_anchor_ratio() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: Some(120_000),
        }
    )));
    assert_eq!(queue.zoom_steps_delta, 1);
    assert_eq!(queue.zoom_anchor_ratio_micros, Some(120_000));

    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
            anchor_ratio_micros: Some(730_000),
        }
    )));
    assert_eq!(queue.zoom_steps_delta, 3);
    assert_eq!(queue.zoom_anchor_ratio_micros, Some(730_000));

    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: false,
            steps: 3,
            anchor_ratio_micros: Some(500_000),
        }
    )));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert_eq!(queue.zoom_anchor_ratio_micros, None);
}
