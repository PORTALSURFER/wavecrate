use super::*;

/// Retained browser-frame materialization must copy manual viewport state.
#[test]
fn projection_segment_browser_frame_copies_manual_viewport_state() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);

    controller.ui.browser.selection.autoscroll = false;
    controller.ui.browser.viewport.view_window_start = 37;
    controller.ui.browser.viewport.render_window_start = 37;

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::BROWSER_FRAME | NativeDirtySegments::BROWSER_ROWS_WINDOW
        )
    );
    assert!(!model.browser.autoscroll);
    assert_eq!(model.browser.view_start_row, 37);
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

#[test]
fn projection_overlay_only_miss_reuses_unique_snapshot_arc() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let first_ptr = Arc::as_ptr(&first_model);
    drop(first_model);

    controller.ui.progress.visible = true;
    controller.ui.progress.modal = true;
    controller.ui.progress.completed = 1;
    controller.ui.progress.total = 3;

    let (second_model, dirty_segments) = cache.resolve_or_project(&mut controller);

    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert_eq!(Arc::as_ptr(&second_model), first_ptr);
    assert!(second_model.progress_overlay.visible);
}

#[test]
fn projection_overlay_only_miss_clones_when_prior_snapshot_is_aliased() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    assert!(!first_model.progress_overlay.visible);

    controller.ui.progress.visible = true;
    controller.ui.progress.modal = true;
    controller.ui.progress.completed = 2;
    controller.ui.progress.total = 5;

    let (second_model, dirty_segments) = cache.resolve_or_project(&mut controller);

    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert!(!Arc::ptr_eq(&first_model, &second_model));
    assert!(!first_model.progress_overlay.visible);
    assert!(second_model.progress_overlay.visible);
}

/// Status-key misses should still refresh selected-column metadata.
#[test]
fn projection_status_miss_updates_selected_column_without_static_dirty() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    assert_eq!(first_model.selected_column, 1);

    controller.ui.browser.selection.selected = Some(SampleBrowserIndex {
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
        crate::app_core::app_api::state::ProgressTaskKind::Normalization,
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
        gui_test_recorder: None,
        last_action_handled: None,
        runtime_exit_emitted: false,
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
