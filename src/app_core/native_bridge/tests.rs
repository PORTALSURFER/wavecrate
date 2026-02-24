use super::{
    DerivedNodeId, NativeProjectionCache, PendingWaveformActions, SempalNativeBridge,
    build_projection_cache_key, build_waveform_projection_key,
};
use crate::app_core::actions::{
    NativeAppBridge, NativeDirtySegments, NativeSegmentRevisions, NativeUiAction,
};
use crate::app_core::controller::AppController;
use crate::app_core::state::{SampleBrowserSort, SampleBrowserTab, TriageFlagFilter, UpdateStatus};
use crate::waveform::WaveformRenderer;
use std::sync::Arc;

#[test]
fn projection_cache_key_changes_when_map_cache_revision_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.map.cached_points_revision += 1;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_key_changes_when_update_status_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.update.status = UpdateStatus::Checking;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when browser filter enum encoding changes.
fn projection_cache_key_changes_when_browser_filter_encoding_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.filter = TriageFlagFilter::Keep;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when browser sort enum encoding changes.
fn projection_cache_key_changes_when_browser_sort_encoding_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.sort = SampleBrowserSort::PlaybackAgeAsc;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when browser tab enum encoding changes.
fn projection_cache_key_changes_when_browser_tab_encoding_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when normalized volume rounds to a new milli bucket.
fn projection_cache_key_changes_when_volume_milli_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.volume = 0.2001;
    let first = build_projection_cache_key(&controller);
    controller.ui.volume = 0.2009;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Full and segment waveform keys must keep milli conversion behavior aligned.
fn projection_and_waveform_keys_share_waveform_milli_conversion() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.cursor = Some(0.1234);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.4321;
    controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.8, 0.2));
    controller.ui.waveform.view.start = 0.1;
    controller.ui.waveform.view.end = 0.9;

    let full = build_projection_cache_key(&controller);
    let segment = build_waveform_projection_key(&controller);
    assert_eq!(full.waveform_cursor_milli, segment.waveform_cursor_milli);
    assert_eq!(
        full.waveform_playhead_milli,
        segment.waveform_playhead_milli
    );
    assert_eq!(
        full.waveform_selection_start_milli,
        segment.waveform_selection_start_milli
    );
    assert_eq!(
        full.waveform_selection_end_milli,
        segment.waveform_selection_end_milli
    );
    assert_eq!(
        full.waveform_view_start_milli,
        segment.waveform_view_start_milli
    );
    assert_eq!(
        full.waveform_view_end_milli,
        segment.waveform_view_end_milli
    );
}

#[test]
/// Waveform key should change when normalized view-range scalars round to new milli values.
fn waveform_projection_key_changes_when_view_milli_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.view.start = 0.1001;
    controller.ui.waveform.view.end = 0.8001;
    let first = build_waveform_projection_key(&controller);

    controller.ui.waveform.view.start = 0.1009;
    controller.ui.waveform.view.end = 0.8009;
    let second = build_waveform_projection_key(&controller);

    assert_ne!(first, second);
}

#[test]
/// Projection cache keys must change when selected-path revisions change.
fn projection_cache_key_changes_when_selected_path_revision_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.selected_paths = vec![std::path::PathBuf::from("first.wav")];
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.selected_paths = vec![std::path::PathBuf::from("second.wav")];
    controller.mark_browser_selected_paths_changed();
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_reuses_model_when_key_unchanged() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let first = cache.resolve_or_project(&mut controller);
    let second = cache.resolve_or_project(&mut controller);
    assert!(Arc::ptr_eq(&first.0, &second.0));
    assert_eq!(second.1, NativeDirtySegments::empty());

    controller.ui.status.text = String::from("changed");
    let refreshed = cache.resolve_or_project(&mut controller);
    assert!(!Arc::ptr_eq(&second.0, &refreshed.0));
    assert_eq!(refreshed.0.status_text.as_str(), "changed");
}

#[test]
fn projection_cache_invalidate_forces_refresh() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let first = cache.resolve_or_project(&mut controller);
    cache.invalidate();
    let second = cache.resolve_or_project(&mut controller);
    assert!(!Arc::ptr_eq(&first.0, &second.0));
    assert_eq!(
        second.1,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR
                | NativeDirtySegments::BROWSER_FRAME
                | NativeDirtySegments::BROWSER_ROWS_WINDOW
                | NativeDirtySegments::MAP_PANEL
                | NativeDirtySegments::WAVEFORM_OVERLAY
                | NativeDirtySegments::GLOBAL_STATIC
        )
    );
}

/// Initial full projection should bump all static segment revisions.
#[test]
fn pull_model_bumps_segment_revisions_on_first_projection() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
    };

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
    };
    bridge.apply_browser_focus_delta_immediately(1);
    assert_eq!(bridge.projection_cache.app_key, Some(key));
}

/// Waveform preview-class actions should bypass queueing for immediate feedback.
#[test]
fn on_action_applies_waveform_preview_actions_immediately() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
    };

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
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
    };

    bridge.on_action(NativeUiAction::SeekWaveform {
        position_milli: 333,
    });

    assert_eq!(bridge.pending_waveform_actions.seek_milli, Some(333));
}

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
    assert_eq!(queue.seek_milli, Some(220));
    assert_eq!(queue.cursor_milli, Some(420));
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
    assert_eq!(queue.deduped_cursor_milli(), None);
}

/// Zoom-to-selection and zoom-full should override discrete zoom deltas.
#[test]
fn waveform_action_queue_zoom_overrides_delta() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: true,
        steps: 3,
    }));
    assert!(queue.enqueue(&NativeUiAction::ZoomWaveformToSelection));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert!(queue.zoom_to_selection);
    assert!(!queue.zoom_full);

    assert!(queue.enqueue(&NativeUiAction::ZoomWaveformFull));
    assert_eq!(queue.zoom_steps_delta, 0);
    assert!(!queue.zoom_to_selection);
    assert!(queue.zoom_full);
}

/// Clear-selection requests should yield to later explicit range updates.
#[test]
fn waveform_action_queue_selection_range_overrides_clear() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::ClearWaveformSelection));
    assert!(queue.clear_selection);
    assert!(queue.selection_range_milli.is_none());
    assert!(queue.enqueue(&NativeUiAction::SetWaveformSelectionRange {
        start_milli: 120,
        end_milli: 400,
    }));
    assert!(!queue.clear_selection);
    assert_eq!(queue.selection_range_milli, Some((120, 400)));
}

/// Pending queue dirty reasons should distinguish overlay-only from view edits.
#[test]
fn waveform_queue_dirty_reason_matches_enqueued_actions() {
    let mut queue = PendingWaveformActions::default();
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
        position_milli: 400,
    }));
    assert_eq!(
        queue.dirty_reason(),
        super::DirtyReason::WaveformOverlayAction
    );

    assert!(queue.enqueue(&NativeUiAction::ZoomWaveform {
        zoom_in: true,
        steps: 1,
    }));
    assert_eq!(queue.dirty_reason(), super::DirtyReason::WaveformViewAction);
}

/// Overlay-only dirty reasons should skip waveform image refresh work.
#[test]
fn waveform_render_inputs_refresh_policy_skips_overlay_only() {
    assert!(!super::waveform_render_inputs_require_refresh(Some(
        super::DirtyReason::WaveformOverlayAction
    )));
    assert!(super::waveform_render_inputs_require_refresh(Some(
        super::DirtyReason::WaveformViewAction
    )));
    assert!(super::waveform_render_inputs_require_refresh(None));
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
    let mut bridge = SempalNativeBridge {
        controller: AppController::new(WaveformRenderer::new(16, 16), None),
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
    };

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
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
    };

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
    let mut bridge = SempalNativeBridge {
        controller: AppController::new(WaveformRenderer::new(16, 16), None),
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
    };

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

#[cfg(feature = "native-bridge-metrics")]
#[test]
fn parse_bridge_profile_enabled_is_case_insensitive() {
    assert!(super::parse_bridge_profile_enabled("TRUE"));
    assert!(super::parse_bridge_profile_enabled("on"));
    assert!(super::parse_bridge_profile_enabled("Yes"));
    assert!(super::parse_bridge_profile_enabled("  true  "));
    assert!(!super::parse_bridge_profile_enabled("0"));
    assert!(!super::parse_bridge_profile_enabled("no"));
    assert!(!super::parse_bridge_profile_enabled(""));
}

/// Immediate waveform preview parser should accept canonical truthy variants.
#[test]
fn parse_immediate_waveform_preview_is_case_insensitive() {
    assert!(super::parse_immediate_waveform_preview("TRUE"));
    assert!(super::parse_immediate_waveform_preview("on"));
    assert!(super::parse_immediate_waveform_preview("Yes"));
    assert!(super::parse_immediate_waveform_preview("  true  "));
    assert!(!super::parse_immediate_waveform_preview("0"));
    assert!(!super::parse_immediate_waveform_preview("no"));
    assert!(!super::parse_immediate_waveform_preview(""));
}

#[cfg(feature = "native-bridge-metrics")]
#[test]
/// Bridge metrics should record projection cache and waveform refresh decisions.
fn bridge_metrics_track_projection_cache_and_waveform_refresh_paths() {
    let projection_hit_before =
        super::PROJECTION_CACHE_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let projection_miss_before =
        super::PROJECTION_CACHE_MISS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let refresh_apply_before =
        super::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let refresh_skip_before =
        super::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.load(std::sync::atomic::Ordering::Relaxed);

    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut cache = NativeProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);
    let _ = cache.resolve_or_project(&mut controller);

    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
    };
    bridge.controller.mark_derived_source_dirty(
        DerivedNodeId::WaveformState,
        super::DirtyReason::WaveformOverlayAction,
    );
    bridge.flush_derived_updates_before_pull(false);
    bridge.controller.mark_derived_source_dirty(
        DerivedNodeId::WaveformState,
        super::DirtyReason::WaveformViewAction,
    );
    bridge.flush_derived_updates_before_pull(false);

    let projection_hit_after =
        super::PROJECTION_CACHE_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let projection_miss_after =
        super::PROJECTION_CACHE_MISS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let refresh_apply_after =
        super::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let refresh_skip_after =
        super::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.load(std::sync::atomic::Ordering::Relaxed);

    assert!(projection_hit_after >= projection_hit_before.saturating_add(1));
    assert!(projection_miss_after >= projection_miss_before.saturating_add(1));
    assert!(refresh_apply_after >= refresh_apply_before.saturating_add(1));
    assert!(refresh_skip_after >= refresh_skip_before.saturating_add(1));
}
