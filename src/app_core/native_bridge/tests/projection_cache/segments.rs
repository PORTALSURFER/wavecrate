use super::*;

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
