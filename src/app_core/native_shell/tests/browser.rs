use super::*;

/// Selected-column projection should default to the neutral middle column when nothing is focused.
#[test]
fn selected_column_defaults_to_middle_column_without_selection() {
    let ui = UiState::default();
    assert_eq!(selected_column_index(&ui), 1);
}

/// Browser render windows should cap to the configured maximum when no focus hints exist.
#[test]
fn browser_render_window_limits_to_target_size() {
    let (start, len) = browser_render_window(500, None, None, true, 0);
    assert_eq!(start, 0);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
}

/// Browser render windows should keep the current window stable for interior focus changes.
#[test]
fn browser_render_window_keeps_existing_window_for_interior_focus_changes() {
    let (start, len) = browser_render_window(500, Some(250), None, true, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 200);
}

/// Browser render windows should nudge downward when focus enters the third row from the bottom.
#[test]
fn browser_render_window_scrolls_when_focus_reaches_third_row_from_bottom() {
    let (start, len) = browser_render_window(500, Some(453), None, true, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 201);
}

/// Browser render windows should nudge upward when focus enters the third row from the top.
#[test]
fn browser_render_window_scrolls_when_focus_reaches_third_row_from_top() {
    let (start, len) = browser_render_window(500, Some(202), None, true, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 199);
}

/// Browser render windows should keep the fourth row from the top stable.
#[test]
fn browser_render_window_keeps_fourth_row_from_top_stable() {
    let (start, len) = browser_render_window(500, Some(203), None, true, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 200);
}

/// Browser render windows should keep the fourth row from the bottom stable.
#[test]
fn browser_render_window_keeps_fourth_row_from_bottom_stable() {
    let (start, len) = browser_render_window(500, Some(452), None, true, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 200);
}

/// Browser render windows should clamp near the end instead of overrunning visible rows.
#[test]
fn browser_render_window_clamps_near_end_of_visible_rows() {
    let (start, len) = browser_render_window(500, Some(490), None, true, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 238);
}

/// Browser render windows should still honor the hard row cap for very large datasets.
#[test]
fn browser_render_window_limits_large_visible_sets_to_cap() {
    let (start, len) = browser_render_window(1_200, None, None, true, 0);
    assert_eq!(start, 0);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
}

/// Browser render windows should keep interior selections stable and still clamp correctly at the tail.
#[test]
fn browser_render_window_keeps_stable_window_and_tail_clamps_for_large_visible_sets() {
    let (center_start, center_len) = browser_render_window(1_200, Some(800), None, true, 700);
    assert_eq!(center_len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(center_start, 700);

    let (tail_start, tail_len) = browser_render_window(1_200, Some(1_190), None, true, 700);
    assert_eq!(tail_len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(tail_start, 938);
}

/// Browser render windows should still clamp at the hard tail when the focus reaches the last row.
#[test]
fn browser_render_window_clamps_at_tail_for_last_visible_row() {
    let (start, len) = browser_render_window(1_200, Some(1_199), None, true, 700);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 944);
}

/// Manual browser viewport scrolling should preserve the requested top row.
#[test]
fn browser_render_window_keeps_manual_view_start_without_autoscroll() {
    let (start, len) = browser_render_window(500, Some(250), None, false, 212);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 212);
}

/// Rating buckets should map deterministically onto browser columns.
#[test]
fn browser_column_index_maps_rating_buckets() {
    assert_eq!(
        browser_column_index(crate::sample_sources::Rating::TRASH_1),
        0
    );
    assert_eq!(
        browser_column_index(crate::sample_sources::Rating::NEUTRAL),
        1
    );
    assert_eq!(
        browser_column_index(crate::sample_sources::Rating::KEEP_1),
        2
    );
}

/// Browser projection should surface sort/tab/search chrome without requiring visible rows.
#[test]
fn browser_projection_exposes_sort_tab_and_search_hint_labels() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.search.sort = SampleBrowserSort::PlaybackAgeDesc;
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 42 };
    let projected = project_browser_model(&mut controller);
    assert_eq!(
        projected.search_placeholder.as_deref(),
        Some("Search samples (Ctrl+F)")
    );
    assert_eq!(projected.sort_label.as_deref(), Some("Playback age ↓"));
    assert_eq!(
        projected.active_tab_label.as_deref(),
        Some("Similarity map")
    );
    assert!(projected.rows.is_empty());
    assert_eq!(projected.visible_count, 42);
}

/// Browser projection should expose focused search placeholder copy when focus is requested.
#[test]
fn browser_projection_marks_search_placeholder_when_focused() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.search.search_focus_requested = true;
    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.search_placeholder.as_deref(), Some("▌"));
}

/// Browser projection should expose manual viewport state for native scrollbar rendering.
#[test]
fn browser_projection_exposes_manual_viewport_state() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.selection.autoscroll = false;
    controller.ui.browser.viewport.view_window_start = 1_470;
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 1_506 };

    let projected = project_browser_panel_frame_model(&controller);

    assert!(!projected.autoscroll);
    assert_eq!(projected.view_start_row, 1_470);
    assert_eq!(projected.visible_count, 1_506);
}

/// Browser chrome projection should expose the toolbar copy shown in the native shell.
#[test]
fn browser_chrome_projection_exposes_toolbar_and_tab_copy() {
    let mut ui = UiState::default();
    ui.browser.search.sort = SampleBrowserSort::Similarity;
    ui.browser.search.similarity_sort_follow_loaded = true;
    let projected = project_browser_chrome_model(&ui, 1437);
    assert_eq!(projected.samples_tab_label, "Samples");
    assert_eq!(projected.map_tab_label, "Similarity map");
    assert_eq!(projected.search_prefix_label, "Search");
    assert_eq!(projected.search_placeholder, "Search samples (Ctrl+F)");
    assert_eq!(projected.activity_ready_label, "Ready");
    assert_eq!(projected.activity_busy_label, "Filtering");
    assert_eq!(projected.sort_prefix_label, "Sort");
    assert_eq!(projected.sort_order_label, "Similarity");
    assert_eq!(projected.similarity_toggle_label, "follow loaded");
    assert_eq!(projected.item_count_label, "1437 items");
}

/// Browser chrome should include focused search copy and caret hint when search is focused.
#[test]
fn browser_chrome_projection_marks_search_focus_copy() {
    let mut ui = UiState::default();
    ui.browser.search.search_focus_requested = true;
    let projected = project_browser_chrome_model(&ui, 7);
    assert_eq!(projected.search_prefix_label, "Search • focused");
    assert_eq!(projected.search_placeholder, "▌");
}
