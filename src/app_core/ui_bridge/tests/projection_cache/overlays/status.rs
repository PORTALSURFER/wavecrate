use super::super::*;

/// Status-key misses should still refresh selected-column metadata.
#[test]
fn projection_status_miss_updates_selected_column_without_static_dirty() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    assert_eq!(first_model.selected_column, 1);

    controller.ui.browser.selection.selected = Some(projection_fixtures::sample_browser_index(
        TriageFlagColumn::Trash,
        0,
    ));
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
    let mut cache = UiProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);

    controller.show_status_progress(
        projection_fixtures::normalization_progress_task(),
        "Normalizing sample",
        4,
        true,
    );
    let (_, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR | NativeDirtySegments::STATE_OVERLAY
        )
    );

    controller.ui.progress.completed = 2;
    controller.ui.progress.detail = Some(String::from("kick.wav"));
    let (_, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR | NativeDirtySegments::STATE_OVERLAY
        )
    );
}
