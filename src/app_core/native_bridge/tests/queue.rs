use super::*;

/// Queued waveform actions should coalesce to last-write-wins semantics.
#[test]
fn waveform_action_queue_last_write_wins() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
        position_milli: 100,
    }));
    assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
        position_milli: 220,
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
        position_milli: 300,
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
        position_milli: 420,
    }));
    assert_eq!(queue.seek_nanos, Some(220_000_000));
    assert_eq!(queue.cursor_nanos, Some(420_000_000));
}

/// Cursor updates should be dropped when seek targets the same milli value.
#[test]
fn waveform_action_queue_dedupes_cursor_when_seek_matches() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
        position_milli: 420,
    }));
    assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
        position_milli: 420,
    }));
    assert_eq!(queue.deduped_cursor_nanos(), None);
}

/// Precise waveform actions should remain last-write-wins without milli fallback.
#[test]
fn waveform_action_queue_keeps_precise_seek_and_cursor_targets() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::SeekWaveformPrecise {
        position_nanos: 123_456_789,
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursorPrecise {
        position_nanos: 234_567_890,
    }));

    assert_eq!(queue.seek_nanos, Some(123_456_789));
    assert_eq!(queue.cursor_nanos, Some(234_567_890));
}

/// Mixed waveform batches should emit deterministic action order with precedence applied.
#[test]
fn waveform_action_queue_emits_mixed_actions_in_order() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: true,
        steps: 3,
        anchor_ratio_micros: Some(250_000),
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformSelectionRange {
        start_micros: 120_000,
        end_micros: 640_000,
        snap_override: false,
        preserve_view_edge: false,
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformViewCenter {
        center_micros: 500_000,
        center_nanos: None,
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
        position_milli: 410,
    }));
    assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
        position_milli: 900,
    }));

    let mut emitted = Vec::new();
    let count = queue.emit_actions(|action| emitted.push(action));

    assert_eq!(count, 5);
    assert_eq!(
        emitted,
        vec![
            NativeUiAction::ZoomWaveform {
                zoom_in: true,
                steps: 3,
                anchor_ratio_micros: Some(250_000),
            },
            NativeUiAction::SetWaveformSelectionRange {
                start_micros: 120_000,
                end_micros: 640_000,
                snap_override: false,
                preserve_view_edge: false,
            },
            NativeUiAction::SetWaveformViewCenter {
                center_micros: 500_000,
                center_nanos: None,
            },
            NativeUiAction::SetWaveformCursorPrecise {
                position_nanos: 410_000_000,
            },
            NativeUiAction::SeekWaveformPrecise {
                position_nanos: 900_000_000,
            },
        ]
    );
}

#[test]
fn waveform_action_queue_keeps_latest_view_center() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::SetWaveformViewCenter {
        center_micros: 200_000,
        center_nanos: None,
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformViewCenter {
        center_micros: 700_000,
        center_nanos: Some(700_000_123),
    }));
    assert_eq!(queue.view_center_micros, Some(700_000));
    assert_eq!(queue.view_center_nanos, Some(700_000_123));
    assert_eq!(queue.dirty_reason(), DirtyReason::WaveformViewAction);
}

/// Zoom-to-selection and zoom-full should override discrete zoom deltas.
#[test]
fn waveform_action_queue_zoom_overrides_delta() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: true,
        steps: 3,
        anchor_ratio_micros: Some(250_000),
    }));
    assert!(queue.enqueue(&NativeUiAction::ZoomWaveformToSelection));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert_eq!(queue.zoom_anchor_ratio_micros, None);
    assert!(queue.zoom_to_selection);
    assert!(!queue.zoom_full);

    assert!(queue.enqueue(&NativeUiAction::ZoomWaveformFull));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert!(!queue.zoom_to_selection);
    assert!(queue.zoom_full);
}

/// Discrete zoom coalescing should keep the most recent pointer anchor.
#[test]
fn waveform_action_queue_keeps_latest_zoom_anchor_ratio() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: true,
        steps: 1,
        anchor_ratio_micros: Some(120_000),
    }));
    assert_eq!(queue.zoom_steps_delta, 1);
    assert_eq!(queue.zoom_anchor_ratio_micros, Some(120_000));

    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: true,
        steps: 2,
        anchor_ratio_micros: Some(730_000),
    }));
    assert_eq!(queue.zoom_steps_delta, 3);
    assert_eq!(queue.zoom_anchor_ratio_micros, Some(730_000));

    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: false,
        steps: 3,
        anchor_ratio_micros: Some(500_000),
    }));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert_eq!(queue.zoom_anchor_ratio_micros, None);
}

/// Clear-selection requests should yield to later explicit range updates.
#[test]
fn waveform_action_queue_selection_range_overrides_clear() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::ClearWaveformSelection));
    assert!(queue.clear_selection);
    assert!(queue.selection_range_micros.is_none());
    assert!(queue.enqueue(&NativeUiAction::SetWaveformSelectionRange {
        start_micros: 120_000,
        end_micros: 400_000,
        snap_override: false,
        preserve_view_edge: false,
    }));
    assert!(!queue.clear_selection);
    assert_eq!(queue.selection_range_micros, Some((120_000, 400_000)));
}

#[test]
fn waveform_action_queue_keeps_smart_scale_selection_as_view_action() {
    let mut queue = PendingWaveformActions::default();
    assert!(
        queue.enqueue(&NativeUiAction::SetWaveformSelectionRangeSmartScale {
            start_micros: 120_000,
            end_micros: 640_000,
        })
    );
    assert_eq!(queue.selection_range_micros, Some((120_000, 640_000)));
    assert!(queue.selection_smart_scale);
    assert_eq!(queue.dirty_reason(), DirtyReason::WaveformViewAction);
    assert_eq!(
        queue.selection_action(),
        Some(NativeUiAction::SetWaveformSelectionRangeSmartScale {
            start_micros: 120_000,
            end_micros: 640_000,
        })
    );
}

#[test]
fn waveform_action_queue_preserves_selection_snap_override() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::SetWaveformSelectionRange {
        start_micros: 120_000,
        end_micros: 640_000,
        snap_override: true,
        preserve_view_edge: false,
    }));

    assert_eq!(
        queue.selection_action(),
        Some(NativeUiAction::SetWaveformSelectionRange {
            start_micros: 120_000,
            end_micros: 640_000,
            snap_override: true,
            preserve_view_edge: false,
        })
    );
}

/// Edit-selection actions are applied immediately and must not be coalesced.
#[test]
fn waveform_action_queue_does_not_absorb_edit_selection_actions() {
    let mut queue = PendingWaveformActions::default();
    assert!(
        !queue.enqueue(&NativeUiAction::SetWaveformEditSelectionRange {
            start_micros: 140_000,
            end_micros: 460_000,
            preserve_view_edge: false,
        })
    );
    assert!(!queue.enqueue(&NativeUiAction::SetWaveformEditFadeInEnd {
        position_micros: 300_000,
    }));
    assert!(
        !queue.enqueue(&NativeUiAction::SetWaveformEditFadeOutStart {
            position_micros: 690_000,
        })
    );
    assert!(!queue.enqueue(&NativeUiAction::FinishWaveformEditFadeDrag));
    assert!(!queue.enqueue(&NativeUiAction::FinishWaveformSelectionRangeDrag));
    assert!(!queue.enqueue(&NativeUiAction::FinishWaveformEditSelectionDrag));
    assert!(!queue.enqueue(&NativeUiAction::ClearWaveformEditSelection));
    assert!(!queue.enqueue(&NativeUiAction::ClearWaveformSelections));
    assert!(!queue.has_pending());
}

/// Pending queue dirty reasons should distinguish overlay-only from view edits.
#[test]
fn waveform_queue_dirty_reason_matches_enqueued_actions() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
        position_milli: 400,
    }));
    assert_eq!(queue.dirty_reason(), DirtyReason::WaveformOverlayAction);

    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: true,
        steps: 1,
        anchor_ratio_micros: None,
    }));
    assert_eq!(queue.dirty_reason(), DirtyReason::WaveformViewAction);
}

/// Overlay-only dirty reasons should skip waveform image refresh work.
#[test]
fn waveform_render_inputs_refresh_policy_skips_overlay_only() {
    assert!(!waveform_render_inputs_require_refresh(Some(
        DirtyReason::WaveformOverlayAction
    )));
    assert!(waveform_render_inputs_require_refresh(Some(
        DirtyReason::WaveformViewAction
    )));
    assert!(waveform_render_inputs_require_refresh(None));
}
