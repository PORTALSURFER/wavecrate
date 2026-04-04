use super::*;

/// Cursor preview actions should coalesce through the waveform queue.
#[test]
fn on_action_queues_waveform_cursor_preview_actions() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SetWaveformCursor {
        position_milli: 420,
    });

    assert_eq!(
        bridge.pending_waveform_actions.cursor_nanos,
        Some(420_000_000)
    );
    assert_eq!(
        bridge.pending_model_pull_preparation,
        PendingModelPullPreparation::LocalOnly
    );
    assert!(!bridge.controller.has_dirty_derived_nodes());
}

/// Edit-preview actions should still apply immediately when they cannot be queued safely.
#[test]
fn on_action_applies_waveform_edit_preview_actions_immediately() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SetWaveformEditSelectionRange {
        start_micros: 120_000,
        end_micros: 640_000,
        preserve_view_edge: false,
    });

    assert!(!bridge.pending_waveform_actions.has_pending());
    assert!(bridge.controller.ui.waveform.edit_selection.is_some());
}

/// Seek actions should remain coalesced in the queue to cap apply-stage cost.
#[test]
fn on_action_keeps_seek_actions_queued() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SeekWaveform {
        position_milli: 333,
    });

    assert_eq!(
        bridge.pending_waveform_actions.seek_nanos,
        Some(333_000_000)
    );
}

/// Overlay-only waveform batches should clear queue state and stay on the local pull path.
#[test]
fn flush_pending_waveform_actions_clears_queue_and_marks_waveform_dirty() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let cache = NativeProjectionCache {
        app_key: Some(build_projection_cache_key(&controller)),
        ..NativeProjectionCache::default()
    };
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: cache,
        projection_key_snapshot: None,
        derived_projection_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
        gui_test_recorder: None,
        last_action_handled: None,
        runtime_exit_emitted: false,
    };

    assert!(
        bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
            position_milli: 500,
        })
    );
    bridge.flush_pending_waveform_actions();

    assert!(!bridge.pending_waveform_actions.has_pending());
    assert!(!bridge.controller.has_dirty_derived_nodes());
    assert!(bridge.projection_cache.app_key.is_none());
    assert_eq!(
        bridge.pending_model_pull_preparation,
        PendingModelPullPreparation::LocalOnly
    );
}

/// No-op queued waveform actions should not dirty the derived graph.
#[test]
fn flush_pending_waveform_actions_noop_skips_dirty_marking() {
    let mut bridge = test_bridge(16);

    assert!(
        bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
            position_milli: 500,
        })
    );
    bridge.flush_pending_waveform_actions();
    let Some(first_snapshot) = bridge.projection_key_snapshot.as_ref().cloned() else {
        panic!("waveform flush should retain a projection key snapshot");
    };
    bridge.flush_derived_updates_before_pull(false);
    assert!(!bridge.controller.has_dirty_derived_nodes());

    assert!(
        bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
            position_milli: 500,
        })
    );
    bridge.flush_pending_waveform_actions();

    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
    assert_eq!(
        bridge.projection_key_snapshot.as_ref(),
        Some(&first_snapshot)
    );
}

/// View-changing queued waveform batches should refresh the cached derived snapshot once.
#[test]
fn flush_pending_waveform_actions_reuses_derived_snapshot_for_view_updates() {
    let mut bridge = test_bridge(16);

    assert!(bridge.enqueue_waveform_action(&NativeUiAction::ZoomWaveformFull));
    bridge.flush_pending_waveform_actions();

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
    let Some(snapshot_key) = bridge.projection_key_snapshot.as_ref() else {
        panic!("waveform flush should retain a projection key snapshot");
    };
    let Some(derived) = bridge.derived_projection_snapshot.as_ref() else {
        panic!("waveform flush should retain a derived projection snapshot");
    };
    assert_eq!(snapshot_key, &derived.app_key);
}
