use super::*;
use std::sync::Arc;

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

/// Exact-position waveform click-play should still dirty the transport graph.
#[test]
fn mark_dirty_for_waveform_click_play_marks_transport_state_dirty() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::PlayWaveformAtPrecise {
        position_nanos: 420_000_000,
    });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::TransportState)
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

/// Folder-row interactions should stay on the sidebar/browser dirty lane.
#[test]
fn mark_dirty_for_folder_actions_stays_targeted() {
    let actions = [
        NativeUiAction::SetFolderSearch {
            pane: None,
            query: String::from("drums"),
        },
        NativeUiAction::ActivateFolderRow {
            pane: None,
            index: 0,
        },
    ];

    for action in actions {
        let mut bridge = test_bridge(16);
        bridge.mark_dirty_for_action(&action);

        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::BrowserState),
            "browser state should be dirty for {action:?}"
        );
        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey),
            "projection key should be dirty for {action:?}"
        );
        assert!(
            !bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::MapState),
            "map state should stay clean for {action:?}"
        );
        assert!(
            !bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::TransportState),
            "transport state should stay clean for {action:?}"
        );
        assert!(
            !bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::StatusState),
            "status state should stay clean for {action:?}"
        );
    }
}

/// Browser review mutation actions should dirty waveform state alongside browser state.
#[test]
fn mark_dirty_for_browser_review_actions_marks_waveform_state_too() {
    let actions = [
        NativeUiAction::ToggleBrowserSampleMark,
        NativeUiAction::AdjustSelectedBrowserRating { delta: 1 },
        NativeUiAction::TagBrowserSelection {
            target: radiant::app::BrowserTagTarget::Keep,
        },
    ];

    for action in actions {
        let mut bridge = test_bridge(16);
        bridge.mark_dirty_for_action(&action);

        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::BrowserState),
            "browser state should be dirty for {action:?}"
        );
        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState),
            "waveform state should be dirty for {action:?}"
        );
        assert!(
            bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey),
            "projection key should be dirty for {action:?}"
        );
        assert!(
            !bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::MapState),
            "map state should stay clean for {action:?}"
        );
        assert!(
            !bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::TransportState),
            "transport state should stay clean for {action:?}"
        );
        assert!(
            !bridge
                .controller
                .is_derived_node_dirty_for_test(DerivedNodeId::StatusState),
            "status state should stay clean for {action:?}"
        );
    }
}

/// Non-targeted actions should keep broad invalidation as the conservative fallback.
#[test]
fn mark_dirty_for_unclassified_action_keeps_broad_invalidation() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::OpenSourceFolderRow {
        pane: None,
        index: 0,
    });

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
        gui_test_recorder: None,
        last_action_handled: None,
        runtime_exit_emitted: false,
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
