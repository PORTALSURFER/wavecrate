use super::super::*;

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
