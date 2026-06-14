use super::super::*;

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
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}
