use super::super::*;

/// The retained projection registry is the source of truth for segment dirty masks.
#[test]
fn retained_segment_handler_plan_matches_dirty_segment_contract() {
    assert_eq!(
        retained_segment_handler_plan(),
        vec![
            (
                ProjectionSegment::StatusBar,
                NativeDirtySegments::STATUS_BAR
            ),
            (
                ProjectionSegment::BrowserFrame,
                NativeDirtySegments::BROWSER_FRAME
            ),
            (
                ProjectionSegment::BrowserTagSidebar,
                NativeDirtySegments::BROWSER_FRAME
            ),
            (
                ProjectionSegment::BrowserRowsWindow,
                NativeDirtySegments::BROWSER_ROWS_WINDOW
            ),
            (ProjectionSegment::MapPanel, NativeDirtySegments::MAP_PANEL),
            (
                ProjectionSegment::WaveformOverlay,
                NativeDirtySegments::WAVEFORM_OVERLAY
            ),
        ]
    );
}

/// No-op pulls should report all retained segment hits and no dirty mask bits.
#[test]
fn projection_segment_noop_pull_hits_all_segments() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|_| {});
    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}
