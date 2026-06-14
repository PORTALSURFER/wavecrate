use super::*;

/// Mixed waveform batches should emit deterministic action order with precedence applied.
#[test]
fn waveform_action_queue_emits_mixed_actions_in_order() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 3,
            anchor_ratio_micros: Some(250_000),
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 120_000,
            end_micros: 640_000,
            snap_override: false,
            preserve_view_edge: false,
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
            center_micros: 500_000,
            center_nanos: None,
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
            position_nanos: 410_000_000,
        },
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 900_000_000,
        },
    )));

    let mut emitted = Vec::new();
    let count = queue.emit_actions(|action| emitted.push(action));

    assert_eq!(count, 5);
    assert_eq!(
        emitted,
        vec![
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                    zoom_in: true,
                    steps: 3,
                    anchor_ratio_micros: Some(250_000),
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                    start_micros: 120_000,
                    end_micros: 640_000,
                    snap_override: false,
                    preserve_view_edge: false,
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
                    center_micros: 500_000,
                    center_nanos: None,
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
                    position_nanos: 410_000_000,
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
                    position_nanos: 900_000_000,
                }
            ),
        ]
    );
}

#[test]
fn waveform_action_queue_commits_selection_before_later_zoom() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 120_000,
            end_micros: 640_000,
            snap_override: false,
            preserve_view_edge: false,
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
            anchor_ratio_micros: Some(250_000),
        }
    )));

    let mut emitted = Vec::new();
    let count = queue.emit_actions(|action| emitted.push(action));

    assert_eq!(count, 2);
    assert_eq!(
        emitted,
        vec![
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                    start_micros: 120_000,
                    end_micros: 640_000,
                    snap_override: false,
                    preserve_view_edge: false,
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                    zoom_in: true,
                    steps: 2,
                    anchor_ratio_micros: Some(250_000),
                }
            ),
        ]
    );
}

#[test]
fn waveform_action_queue_applies_zoom_before_later_selection() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
            anchor_ratio_micros: Some(250_000),
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 120_000,
            end_micros: 640_000,
            snap_override: false,
            preserve_view_edge: false,
        }
    )));

    let mut emitted = Vec::new();
    let count = queue.emit_actions(|action| emitted.push(action));

    assert_eq!(count, 2);
    assert_eq!(
        emitted,
        vec![
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                    zoom_in: true,
                    steps: 2,
                    anchor_ratio_micros: Some(250_000),
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                    start_micros: 120_000,
                    end_micros: 640_000,
                    snap_override: false,
                    preserve_view_edge: false,
                }
            ),
        ]
    );
}

#[test]
fn waveform_action_queue_preserves_view_center_order_around_selection() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
            anchor_ratio_micros: Some(750_000),
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 200_000,
            end_micros: 500_000,
            snap_override: true,
            preserve_view_edge: false,
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
            center_micros: 350_000,
            center_nanos: Some(350_000_000),
        }
    )));

    let mut emitted = Vec::new();
    let count = queue.emit_actions(|action| emitted.push(action));

    assert_eq!(count, 3);
    assert_eq!(
        emitted,
        vec![
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                    zoom_in: false,
                    steps: 1,
                    anchor_ratio_micros: Some(750_000),
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                    start_micros: 200_000,
                    end_micros: 500_000,
                    snap_override: true,
                    preserve_view_edge: false,
                }
            ),
            NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
                    center_micros: 350_000,
                    center_nanos: Some(350_000_000),
                }
            ),
        ]
    );
}
