use super::{
    DerivedNodeId, NativeProjectionCache, PendingWaveformActions, SempalNativeBridge,
    build_projection_cache_key, build_waveform_projection_key,
};
use crate::app_core::actions::{
    NativeAppBridge, NativeDirtySegments, NativeSegmentRevisions, NativeUiAction,
};
use crate::app_core::app_api::state::SampleBrowserIndex;
use crate::app_core::controller::AppController;
use crate::app_core::state::{
    MapBounds, MapPoint, MapQueryBounds, SampleBrowserSort, SampleBrowserTab, StatusTone,
    TriageFlagColumn, TriageFlagFilter, UpdateStatus,
};
use crate::waveform::WaveformRenderer;
use std::path::PathBuf;
use std::sync::Arc;

/// Run one retained projection step after warming cache and return dirty mask + lookup counters.
fn project_after_warm_cache(
    mutate: impl FnOnce(&mut AppController),
) -> (NativeDirtySegments, super::ProjectionSegmentLookupCounts) {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);
    let _ = cache.take_segment_lookup_counts();
    mutate(&mut controller);
    let (_, dirty_segments) = cache.resolve_or_project(&mut controller);
    let lookup_counts = cache.take_segment_lookup_counts();
    (dirty_segments, lookup_counts)
}

/// Assert one segment lookup bucket equals the expected hit/miss counters.
fn assert_segment_lookup_counts(
    actual: super::ProjectionSegmentLookupCount,
    expected_hit: u64,
    expected_miss: u64,
) {
    assert_eq!(actual.hit_count, expected_hit);
    assert_eq!(actual.miss_count, expected_miss);
}

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
fn projection_cache_key_changes_when_options_panel_state_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.options_panel.open = true;
    controller.ui.trash_folder = Some(PathBuf::from("trash_bin"));
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
fn projection_cache_key_changes_when_browser_view_window_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.autoscroll = false;
    controller.ui.browser.view_window_start = 7;
    controller.ui.browser.render_window_start = 7;
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
/// Full and segment waveform keys must keep static waveform milli conversion aligned.
fn projection_and_waveform_keys_share_waveform_milli_conversion() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.8, 0.2));
    controller.ui.waveform.edit_selection = Some(
        crate::selection::SelectionRange::new(0.7, 0.4)
            .with_fade_in(0.2, 0.8)
            .with_fade_in_mute(0.1)
            .with_fade_out(0.3, 0.2)
            .with_fade_out_mute(0.2),
    );
    controller.ui.waveform.view.start = 0.1;
    controller.ui.waveform.view.end = 0.9;

    let full = build_projection_cache_key(&controller);
    let segment = build_waveform_projection_key(&controller);
    assert_eq!(
        full.waveform_selection_start_milli,
        segment.waveform_selection_start_milli
    );
    assert_eq!(
        full.waveform_selection_end_milli,
        segment.waveform_selection_end_milli
    );
    assert_eq!(
        full.waveform_edit_selection_start_milli,
        segment.waveform_edit_selection_start_milli
    );
    assert_eq!(
        full.waveform_edit_selection_end_milli,
        segment.waveform_edit_selection_end_milli
    );
    assert_eq!(
        full.waveform_edit_fade_in_end_milli,
        segment.waveform_edit_fade_in_end_milli
    );
    assert_eq!(
        full.waveform_edit_fade_in_mute_start_milli,
        segment.waveform_edit_fade_in_mute_start_milli
    );
    assert_eq!(
        full.waveform_edit_fade_in_curve_milli,
        segment.waveform_edit_fade_in_curve_milli
    );
    assert_eq!(
        full.waveform_edit_fade_out_start_milli,
        segment.waveform_edit_fade_out_start_milli
    );
    assert_eq!(
        full.waveform_edit_fade_out_mute_end_milli,
        segment.waveform_edit_fade_out_mute_end_milli
    );
    assert_eq!(
        full.waveform_edit_fade_out_curve_milli,
        segment.waveform_edit_fade_out_curve_milli
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
/// Cursor/playhead motion should not invalidate static projection keys.
fn projection_and_waveform_keys_ignore_cursor_and_playhead_motion() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first_full = build_projection_cache_key(&controller);
    let first_waveform = build_waveform_projection_key(&controller);

    controller.ui.waveform.cursor = Some(0.1234);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.4321;

    let second_full = build_projection_cache_key(&controller);
    let second_waveform = build_waveform_projection_key(&controller);
    assert_eq!(first_full, second_full);
    assert_eq!(first_waveform, second_waveform);
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
/// Waveform option toggles must invalidate both full and waveform segment projection keys.
fn waveform_option_toggles_change_projection_and_waveform_keys() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first_full = build_projection_cache_key(&controller);
    let first_waveform = build_waveform_projection_key(&controller);

    controller.ui.waveform.channel_view = crate::waveform::WaveformChannelView::SplitStereo;
    controller.ui.waveform.normalized_audition_enabled = true;
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.transient_snap_enabled = true;
    controller.ui.waveform.transient_markers_enabled = false;
    controller.ui.waveform.slice_mode_enabled = true;

    let second_full = build_projection_cache_key(&controller);
    let second_waveform = build_waveform_projection_key(&controller);
    assert_ne!(first_full, second_full);
    assert_ne!(first_waveform, second_waveform);
}

#[test]
/// Projection cache keys must change when selected-path revisions change.
fn projection_cache_key_changes_when_selected_path_revision_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.selected_paths = vec![std::path::PathBuf::from("first.wav")];
    controller.mark_browser_selected_paths_changed();
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

    controller.set_status("changed", StatusTone::Info);
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

#[test]
/// Async revision-bus updates must invalidate the retained projection key on first pull.
fn bridge_reprojects_after_async_loaded_wav_revision_change() {
    let controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };

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
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
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
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };
    bridge.apply_browser_focus_delta_immediately(1);
    assert_eq!(bridge.projection_cache.app_key, Some(key));
}

/// Local-only focus actions should avoid dirtying derived state and arm the
/// one-shot local pull fast path.
#[test]
fn local_focus_actions_arm_local_model_pull_fast_path() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };

    bridge.on_action(NativeUiAction::FocusBrowserSearch);

    assert_eq!(
        bridge.pending_model_pull_preparation,
        super::PendingModelPullPreparation::LocalOnly
    );
    assert!(!bridge.controller.has_dirty_derived_nodes());
}

/// Browser search mutations still require the full pull-preparation path.
#[test]
fn search_query_actions_stay_on_full_model_pull_preparation() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };

    bridge.on_action(NativeUiAction::SetBrowserSearch {
        query: String::from("kick"),
    });

    assert_eq!(
        bridge.pending_model_pull_preparation,
        super::PendingModelPullPreparation::Full
    );
    assert!(bridge.controller.has_dirty_derived_nodes());
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
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
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
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
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
        start_milli: 120,
        end_milli: 640,
        preserve_view_edge: false,
    }));
    assert!(queue.enqueue(&NativeUiAction::SetWaveformCursor {
        position_milli: 410,
    }));
    assert!(queue.enqueue(&NativeUiAction::SeekWaveform {
        position_milli: 900,
    }));

    let mut emitted = Vec::new();
    let count = queue.emit_actions(|action| emitted.push(action));

    assert_eq!(count, 4);
    assert_eq!(
        emitted,
        vec![
            NativeUiAction::ZoomWaveform {
                zoom_in: true,
                steps: 3,
                anchor_ratio_micros: Some(250_000),
            },
            NativeUiAction::SetWaveformSelectionRange {
                start_milli: 120,
                end_milli: 640,
                preserve_view_edge: false,
            },
            NativeUiAction::SetWaveformCursor {
                position_milli: 410,
            },
            NativeUiAction::SeekWaveform {
                position_milli: 900,
            },
        ]
    );
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
    assert!(queue.selection_range_milli.is_none());
    assert!(queue.enqueue(&NativeUiAction::SetWaveformSelectionRange {
        start_milli: 120,
        end_milli: 400,
        preserve_view_edge: false,
    }));
    assert!(!queue.clear_selection);
    assert_eq!(queue.selection_range_milli, Some((120, 400)));
}

#[test]
fn waveform_action_queue_keeps_smart_scale_selection_as_view_action() {
    let mut queue = PendingWaveformActions::default();
    assert!(
        queue.enqueue(&NativeUiAction::SetWaveformSelectionRangeSmartScale {
            start_milli: 120,
            end_milli: 640,
        })
    );
    assert_eq!(queue.selection_range_milli, Some((120, 640)));
    assert!(queue.selection_smart_scale);
    assert_eq!(queue.dirty_reason(), super::DirtyReason::WaveformViewAction);
    assert_eq!(
        queue.selection_action(),
        Some(NativeUiAction::SetWaveformSelectionRangeSmartScale {
            start_milli: 120,
            end_milli: 640,
        })
    );
}

/// Edit-selection actions are applied immediately and must not be coalesced.
#[test]
fn waveform_action_queue_does_not_absorb_edit_selection_actions() {
    let mut queue = PendingWaveformActions::default();
    assert!(
        !queue.enqueue(&NativeUiAction::SetWaveformEditSelectionRange {
            start_milli: 140,
            end_milli: 460,
            preserve_view_edge: false,
        })
    );
    assert!(!queue.enqueue(&NativeUiAction::SetWaveformEditFadeInEnd {
        position_milli: 300,
    }));
    assert!(
        !queue.enqueue(&NativeUiAction::SetWaveformEditFadeOutStart {
            position_milli: 690,
        })
    );
    assert!(!queue.enqueue(&NativeUiAction::FinishWaveformEditFadeDrag));
    assert!(!queue.enqueue(&NativeUiAction::ClearWaveformEditSelection));
    assert!(!queue.has_pending());
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
        anchor_ratio_micros: None,
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
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
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
    let mut bridge = SempalNativeBridge {
        controller: AppController::new(WaveformRenderer::new(16, 16), None),
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
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
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
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

/// High-frequency browser focus actions should avoid broad invalidation fan-out.
#[test]
fn mark_dirty_for_browser_focus_action_stays_targeted() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };

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
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
    };

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
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
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
    let mut bridge = SempalNativeBridge {
        controller: AppController::new(WaveformRenderer::new(16, 16), None),
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
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

/// No-op pulls should report all retained segment hits and no dirty mask bits.
#[test]
fn projection_segment_noop_pull_hits_all_segments() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|_| {});
    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Status-key changes should rematerialize only the status segment.
#[test]
fn projection_segment_status_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.projection_revisions.status =
            controller.ui.projection_revisions.status.wrapping_add(1);
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATUS_BAR)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Browser-frame changes should stay isolated from browser-row window materialization.
#[test]
fn projection_segment_browser_frame_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.browser.sort = SampleBrowserSort::PlaybackAgeAsc;
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::BROWSER_FRAME)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Retained browser-frame materialization must copy active rating-filter flags.
#[test]
fn projection_segment_browser_frame_copies_active_rating_filters() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);

    controller.ui.browser.rating_filter.insert(3);
    controller.ui.browser.rating_filter.insert(4);
    controller.mark_browser_search_projection_revision_dirty();

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR | NativeDirtySegments::BROWSER_FRAME
        )
    );
    assert!(model.browser.active_rating_filters[6]);
    assert_eq!(
        model.browser.active_rating_filters,
        [false, false, false, false, false, false, true, true]
    );
}

/// Browser row-revision changes should only rematerialize browser rows.
#[test]
fn projection_segment_browser_rows_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.mark_browser_selected_paths_changed();
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::BROWSER_ROWS_WINDOW)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 0, 1);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Map-key changes should rematerialize only the map segment.
#[test]
fn projection_segment_map_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.map.cached_points_revision =
            controller.ui.map.cached_points_revision.wrapping_add(1);
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::MAP_PANEL)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 0, 1);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Map selection-only changes should rematerialize the segment while reusing retained points.
#[test]
fn projection_segment_map_selection_dirty_reuses_retained_point_arc() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
    controller.ui.map.umap_version = String::from("v1");
    controller.ui.map.bounds = Some(MapBounds {
        min_x: -1.0,
        max_x: 1.0,
        min_y: -1.0,
        max_y: 1.0,
    });
    controller.ui.map.cached_bounds_source_id = None;
    controller.ui.map.cached_bounds_umap_version = Some(String::from("v1"));
    controller.ui.map.last_query = Some(MapQueryBounds {
        min_x: -1.0,
        max_x: 1.0,
        min_y: -1.0,
        max_y: 1.0,
    });
    controller.ui.map.cached_points = vec![MapPoint {
        sample_id: Arc::<str>::from("source::kick.wav"),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }];
    controller.ui.map.cached_points_source_id = None;
    controller.ui.map.cached_points_umap_version = Some(String::from("v1"));
    controller.ui.map.cached_points_revision = 7;

    let mut cache = NativeProjectionCache::default();
    let (first, _) = cache.resolve_or_project(&mut controller);
    let _ = cache.take_segment_lookup_counts();

    controller.ui.map.selected_sample_id = Some(String::from("source::kick.wav"));
    controller.mark_map_selection_projection_revision_dirty();

    let (second, dirty_segments) = cache.resolve_or_project(&mut controller);
    let lookup_counts = cache.take_segment_lookup_counts();

    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::MAP_PANEL)
    );
    assert!(Arc::ptr_eq(&first.map.points, &second.map.points));
    assert_eq!(
        second.map.selected_sample_id.as_deref(),
        Some("source::kick.wav")
    );
    assert_segment_lookup_counts(lookup_counts.map_panel, 0, 1);
}

/// Waveform-key changes should rematerialize only the waveform segment.
#[test]
fn projection_segment_waveform_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.waveform.view.start = 0.25;
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::WAVEFORM_OVERLAY)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 0, 1);
}

/// Cursor-only updates should stay on motion overlays and keep static segments cached.
#[test]
fn projection_segment_cursor_motion_keeps_static_segments_cached() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.waveform.cursor = Some(0.25);
    });
    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Non-segment static-key changes should only set the global static dirty bit.
#[test]
fn projection_segment_non_segment_static_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.volume = 0.75;
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::GLOBAL_STATIC)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Prompt/progress/drag app-key misses should not flip static dirty segments.
#[test]
fn projection_segment_overlay_only_changes_keep_segment_hits_and_static_clean() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.progress.visible = true;
        controller.ui.progress.modal = true;
        controller.ui.progress.completed = 2;
        controller.ui.progress.total = 5;
    });
    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Overlay-only misses should preserve retained static fields while refreshing overlays.
#[test]
fn projection_overlay_only_miss_skips_static_non_segment_refresh() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let mut retained = Arc::unwrap_or_clone(first_model);
    retained.sources_label = String::from("sentinel");
    cache.app_model_working = Some(retained.clone());
    cache.app_model = Some(Arc::new(retained));

    controller.ui.progress.visible = true;
    controller.ui.progress.modal = true;
    controller.ui.progress.completed = 1;
    controller.ui.progress.total = 3;

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert_eq!(model.sources_label.as_str(), "sentinel");
    assert!(model.progress_overlay.visible);
}

#[test]
fn projection_overlay_only_miss_refreshes_options_panel_fields() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let mut retained = Arc::unwrap_or_clone(first_model);
    retained.options_panel.visible = false;
    retained.options_panel.trash_folder_label = None;
    cache.app_model_working = Some(retained.clone());
    cache.app_model = Some(Arc::new(retained));

    controller.ui.options_panel.open = true;
    controller.ui.trash_folder = Some(PathBuf::from("trash_bin"));

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert!(model.options_panel.visible);
    assert_eq!(
        model.options_panel.trash_folder_label.as_deref(),
        Some("trash_bin")
    );
}

/// Status-key misses should still refresh selected-column metadata.
#[test]
fn projection_status_miss_updates_selected_column_without_static_dirty() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    assert_eq!(first_model.selected_column, 1);

    controller.ui.browser.selected = Some(SampleBrowserIndex {
        column: TriageFlagColumn::Trash,
        row: 0,
    });
    controller.ui.projection_revisions.status =
        controller.ui.projection_revisions.status.wrapping_add(1);

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(model.selected_column, 0);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATUS_BAR)
    );
}

/// Non-modal progress updates should invalidate the retained status segment.
#[test]
fn projection_status_segment_refreshes_for_footer_progress_updates() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);

    controller.show_status_progress(
        crate::app::state::ProgressTaskKind::Normalization,
        "Normalizing sample",
        4,
        true,
    );
    let (_, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATUS_BAR)
    );

    controller.ui.progress.completed = 2;
    controller.ui.progress.detail = Some(String::from("kick.wav"));
    let (_, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATUS_BAR)
    );
}

#[cfg(feature = "native-bridge-metrics")]
#[test]
/// Shared env truthy parsing should accept canonical bridge-profile variants.
fn env_truthy_parser_is_case_insensitive_for_bridge_flags() {
    assert!(crate::env_flags::is_truthy("TRUE"));
    assert!(crate::env_flags::is_truthy("on"));
    assert!(crate::env_flags::is_truthy("Yes"));
    assert!(crate::env_flags::is_truthy("  true  "));
    assert!(!crate::env_flags::is_truthy("0"));
    assert!(!crate::env_flags::is_truthy("no"));
    assert!(!crate::env_flags::is_truthy(""));
}

/// Immediate waveform preview parser should accept canonical truthy variants.
#[test]
fn env_truthy_parser_is_case_insensitive_for_immediate_preview_flag() {
    assert!(crate::env_flags::is_truthy("TRUE"));
    assert!(crate::env_flags::is_truthy("on"));
    assert!(crate::env_flags::is_truthy("Yes"));
    assert!(crate::env_flags::is_truthy("  true  "));
    assert!(!crate::env_flags::is_truthy("0"));
    assert!(!crate::env_flags::is_truthy("no"));
    assert!(!crate::env_flags::is_truthy(""));
}

#[cfg(feature = "native-bridge-metrics")]
#[test]
/// Bridge metrics should record projection cache and waveform refresh decisions.
fn bridge_metrics_track_projection_cache_and_waveform_refresh_paths() {
    let projection_hit_before =
        super::metrics::PROJECTION_CACHE_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let projection_miss_before =
        super::metrics::PROJECTION_CACHE_MISS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let refresh_apply_before = super::metrics::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);
    let refresh_skip_before = super::metrics::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);

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
        pending_model_pull_preparation: super::PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
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
        super::metrics::PROJECTION_CACHE_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let projection_miss_after =
        super::metrics::PROJECTION_CACHE_MISS_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let refresh_apply_after = super::metrics::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);
    let refresh_skip_after = super::metrics::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);

    assert!(projection_hit_after >= projection_hit_before.saturating_add(1));
    assert!(projection_miss_after >= projection_miss_before.saturating_add(1));
    assert!(refresh_apply_after >= refresh_apply_before.saturating_add(1));
    assert!(refresh_skip_after >= refresh_skip_before.saturating_add(1));
}
