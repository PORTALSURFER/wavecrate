use super::*;

/// Async revision-bus updates must invalidate the retained projection key on first pull.
#[test]
fn bridge_reprojects_after_async_loaded_wav_revision_change() {
    let mut bridge = test_bridge(32);

    let first = bridge.project_model();
    assert!(first.waveform.loaded_label.is_none());

    bridge
        .controller
        .set_ui_loaded_wav(Some(PathBuf::from("fresh_take.wav")));
    let second = bridge.project_model();
    assert_eq!(second.waveform.loaded_label.as_deref(), Some("fresh_take"));
}

/// Initial full projection should bump all static segment revisions.
#[test]
fn pull_model_bumps_segment_revisions_on_first_projection() {
    let mut bridge = test_bridge(16);
    let _ = bridge.pull_model();
    let revisions = bridge.take_segment_revisions();

    assert!(revisions.has_static_revisions());
    assert!(revisions.status_bar > 0);
    assert!(revisions.browser_frame > 0);
    assert!(revisions.browser_rows_window > 0);
    assert!(revisions.map_panel > 0);
    assert!(revisions.waveform_overlay > 0);
    assert!(revisions.global_static > 0);
}

/// No-op immediate focus movement should keep projection cache keys intact.
#[test]
fn apply_browser_focus_delta_immediate_noop_keeps_projection_cache_key() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let key = build_projection_cache_key(&controller);
    let cache = NativeProjectionCache {
        app_key: Some(key.clone()),
        ..NativeProjectionCache::default()
    };

    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: cache,
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };
    bridge.apply_browser_focus_delta_immediately(1);
    assert_eq!(bridge.projection_cache.app_key, Some(key));
}

/// Local-only focus actions should avoid dirtying derived state and arm the one-shot fast path.
#[test]
fn local_focus_actions_arm_local_model_pull_fast_path() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::FocusBrowserSearch);

    assert_eq!(
        bridge.pending_model_pull_preparation,
        PendingModelPullPreparation::LocalOnly
    );
    assert!(!bridge.controller.has_dirty_derived_nodes());
}

/// Browser search mutations still require the full pull-preparation path.
#[test]
fn search_query_actions_stay_on_full_model_pull_preparation() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SetBrowserSearch {
        query: String::from("kick"),
    });

    assert_eq!(
        bridge.pending_model_pull_preparation,
        PendingModelPullPreparation::Full
    );
    assert!(bridge.controller.has_dirty_derived_nodes());
}

/// Waveform preview-class actions should bypass queueing for immediate feedback.
#[test]
fn on_action_applies_waveform_preview_actions_immediately() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SetWaveformCursor {
        position_milli: 420,
    });

    assert_eq!(bridge.pending_waveform_actions.cursor_milli, None);
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
}

/// Seek actions should remain coalesced in the queue to cap apply-stage cost.
#[test]
fn on_action_keeps_seek_actions_queued() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SeekWaveform {
        position_milli: 333,
    });

    assert_eq!(bridge.pending_waveform_actions.seek_milli, Some(333));
}

/// Flushing queued waveform actions should clear queue state and mark waveform dirties.
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
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };

    assert!(
        bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
            position_milli: 500,
        })
    );
    bridge.flush_pending_waveform_actions();

    assert!(!bridge.pending_waveform_actions.has_pending());
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
    assert!(bridge.projection_cache.app_key.is_some());
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

/// Action classification should mark waveform source and projection-key nodes dirty.
#[test]
fn mark_dirty_for_waveform_action_marks_graph_nodes() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::SeekWaveform {
        position_milli: 250,
    });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey)
    );
}

/// High-frequency browser focus actions should avoid broad invalidation fan-out.
#[test]
fn mark_dirty_for_browser_focus_action_stays_targeted() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::MoveBrowserFocus { delta: 1 });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::BrowserState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey)
    );
    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::MapState)
    );
    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::TransportState)
    );
    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::StatusState)
    );
}

/// Non-targeted actions should keep broad invalidation as the conservative fallback.
#[test]
fn mark_dirty_for_unclassified_action_keeps_broad_invalidation() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::OpenSourceFolderRow { index: 0 });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::BrowserState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::MapState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::TransportState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::StatusState)
    );
}

/// Flushing derived updates should clear graph dirties and invalidate projection cache key.
#[test]
fn flush_derived_updates_clears_nodes_and_invalidates_key() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let cache = NativeProjectionCache {
        app_key: Some(build_projection_cache_key(&controller)),
        ..NativeProjectionCache::default()
    };
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: cache,
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };
    let _ = bridge.projection_key_snapshot();
    assert!(bridge.projection_key_snapshot.is_some());

    bridge.mark_dirty_for_action(&NativeUiAction::SetBrowserSearch {
        query: String::from("kick"),
    });
    bridge.flush_derived_updates_before_pull(false);

    assert!(!bridge.controller.has_dirty_derived_nodes());
    assert!(bridge.projection_cache.app_key.is_none());
    assert!(bridge.projection_key_snapshot.is_none());
}

/// Repeated no-op pulls should preserve snapshot/cache reuse and avoid full reprojection.
#[test]
fn pull_model_snapshot_noop_pull_reuses_cached_projection() {
    let mut bridge = test_bridge(16);

    let first_model = bridge.pull_model_arc_snapshot();
    let Some(first_snapshot) = bridge.projection_key_snapshot.as_ref().cloned() else {
        panic!("pull should populate projection key snapshot");
    };
    let Some(first_cache_key) = bridge.projection_cache.app_key.as_ref().cloned() else {
        panic!("pull should populate projection cache key");
    };
    assert_eq!(first_snapshot, first_cache_key);

    let second_model = bridge.pull_model_arc_snapshot();
    assert!(Arc::ptr_eq(&first_model, &second_model));
    assert_eq!(
        bridge.projection_key_snapshot.as_ref(),
        Some(&first_snapshot)
    );
}
