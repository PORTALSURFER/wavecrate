use super::*;

/// Pending queue dirty reasons should distinguish overlay-only from view edits.
#[test]
fn waveform_queue_dirty_reason_matches_enqueued_actions() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
            position_nanos: 400_000_000,
        },
    )));
    assert_eq!(
        queue.dirty_reason(),
        InvalidationReason::WaveformOverlayAction
    );

    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: None,
        }
    )));
    assert_eq!(queue.dirty_reason(), InvalidationReason::WaveformViewAction);
}

/// Overlay-only dirty reasons should skip waveform image refresh work.
#[test]
fn waveform_render_inputs_refresh_policy_skips_overlay_only() {
    assert!(!waveform_render_inputs_require_refresh(Some(
        InvalidationReason::WaveformOverlayAction
    )));
    assert!(waveform_render_inputs_require_refresh(Some(
        InvalidationReason::WaveformViewAction
    )));
    assert!(waveform_render_inputs_require_refresh(None));
}
