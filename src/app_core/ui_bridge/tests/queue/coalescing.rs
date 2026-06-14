use super::*;

/// Queued waveform actions should coalesce to last-write-wins semantics.
#[test]
fn waveform_action_queue_last_write_wins() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 100_000_000,
        },
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 220_000_000,
        },
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
            position_nanos: 300_000_000,
        },
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
            position_nanos: 420_000_000,
        },
    )));
    assert_eq!(queue.seek_nanos, Some(220_000_000));
    assert_eq!(queue.cursor_nanos, Some(420_000_000));
}

/// Cursor updates should be dropped when seek targets the same milli value.
#[test]
fn waveform_action_queue_dedupes_cursor_when_seek_matches() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
            position_nanos: 420_000_000,
        },
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 420_000_000,
        },
    )));
    assert_eq!(queue.deduped_cursor_nanos(), None);
}

/// Precise waveform actions should remain last-write-wins without milli fallback.
#[test]
fn waveform_action_queue_keeps_precise_seek_and_cursor_targets() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 123_456_789,
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
            position_nanos: 234_567_890,
        }
    )));

    assert_eq!(queue.seek_nanos, Some(123_456_789));
    assert_eq!(queue.cursor_nanos, Some(234_567_890));
}

#[test]
fn waveform_action_queue_keeps_latest_view_center() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
            center_micros: 200_000,
            center_nanos: None,
        }
    )));
    assert!(queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
            center_micros: 700_000,
            center_nanos: Some(700_000_123),
        }
    )));
    assert_eq!(queue.view_center_micros, Some(700_000));
    assert_eq!(queue.view_center_nanos, Some(700_000_123));
    assert_eq!(queue.dirty_reason(), InvalidationReason::WaveformViewAction);
}
