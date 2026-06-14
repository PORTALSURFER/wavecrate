use super::super::*;

/// Retained browser-frame materialization must copy manual viewport state.
#[test]
fn projection_segment_browser_frame_copies_manual_viewport_state() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
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
