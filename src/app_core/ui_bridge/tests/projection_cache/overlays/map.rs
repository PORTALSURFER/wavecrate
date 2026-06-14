use super::super::*;

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

    let mut cache = UiProjectionCache::default();
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
        second.map.selected_item_id.as_deref(),
        Some("source::kick.wav")
    );
    assert_segment_lookup_counts(lookup_counts.map_panel, 0, 1);
}
