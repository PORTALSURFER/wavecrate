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

/// Edit-selection actions are applied immediately and must not be coalesced.
#[test]
fn waveform_action_queue_does_not_absorb_edit_selection_actions() {
    let mut queue = PendingWaveformActions::default();
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange {
            start_micros: 140_000,
            end_micros: 460_000,
            preserve_view_edge: false,
        }
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd {
            position_micros: 300_000,
        }
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutStart {
            position_micros: 690_000,
        }
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections
    )));
    assert!(!queue.has_pending());
}

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
