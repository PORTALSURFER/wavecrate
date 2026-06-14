use super::*;

/// Clear-selection requests should yield to later explicit range updates.
#[test]
fn waveform_action_queue_selection_range_overrides_clear() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ClearWaveformSelection
    )));
    assert!(queue.clear_selection);
    assert!(queue.selection_range_micros.is_none());
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 120_000,
            end_micros: 400_000,
            snap_override: false,
            preserve_view_edge: false,
        }
    )));
    assert!(!queue.clear_selection);
    assert_eq!(queue.selection_range_micros, Some((120_000, 400_000)));
}

#[test]
fn waveform_action_queue_keeps_smart_scale_selection_as_view_action() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScale {
            start_micros: 120_000,
            end_micros: 640_000,
        }
    )));
    assert_eq!(queue.selection_range_micros, Some((120_000, 640_000)));
    assert!(queue.selection_smart_scale);
    assert_eq!(queue.dirty_reason(), InvalidationReason::WaveformViewAction);
    assert!(queue.requires_full_model_pull());
    assert_eq!(
        queue.selection_action(),
        Some(NativeUiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScale {
                start_micros: 120_000,
                end_micros: 640_000,
            }
        ))
    );
}

#[test]
fn waveform_action_queue_preserves_selection_snap_override() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 120_000,
            end_micros: 640_000,
            snap_override: true,
            preserve_view_edge: false,
        }
    )));

    assert_eq!(
        queue.selection_action(),
        Some(NativeUiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                start_micros: 120_000,
                end_micros: 640_000,
                snap_override: true,
                preserve_view_edge: false,
            }
        ))
    );
}
