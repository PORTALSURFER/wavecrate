use super::*;

fn browser_projection_test_entry(name: &str) -> crate::sample_sources::WavEntry {
    crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from(name),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }
}

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

/// Browser tag sidebar should fall back to the focused visible row when no path snapshot exists yet.
#[test]
fn browser_projection_sidebar_uses_selected_visible_target_snapshot_fallback() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.tag_sidebar_open = true;
    controller.set_wav_entries_for_tests(vec![
        browser_projection_test_entry("first.wav"),
        browser_projection_test_entry("second.wav"),
    ]);
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 2 };
    controller.ui.browser.selection.selected_visible = Some(1);
    controller.ui.browser.selection.last_focused_path = None;
    controller.ui.browser.selection.selected_paths.clear();

    let projected = project_browser_panel_frame_model(&mut controller);

    assert_eq!(projected.tag_sidebar.selected_count, 1);
    assert_eq!(projected.tag_sidebar.header_label.as_str(), "second.wav");
    assert!(!projected.tag_sidebar.primary_action_enabled);
    controller.ui.browser.tag_sidebar_auto_rename = true;
    let projected = project_browser_panel_frame_model(&mut controller);
    assert!(projected.tag_sidebar.primary_action_enabled);
}

#[test]
fn browser_projection_sidebar_projects_common_normal_tags_from_db_usage() {
    let (mut controller, source) = browser_projection_controller_with_source(vec![
        browser_projection_test_entry("first.wav"),
        browser_projection_test_entry("second.wav"),
    ]);
    let db = controller.database_for(&source).unwrap();
    db.assign_tag_to_path(std::path::Path::new("first.wav"), "Texture")
        .unwrap();
    db.assign_tag_to_path(std::path::Path::new("second.wav"), "Texture")
        .unwrap();
    db.assign_tag_to_path(std::path::Path::new("first.wav"), "Loop")
        .unwrap();
    db.assign_tag_to_path(std::path::Path::new("second.wav"), "Rare FX")
        .unwrap();
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.selection.selected_paths = vec![std::path::PathBuf::from("first.wav")];
    controller.mark_browser_selected_paths_changed();

    let projected = project_browser_panel_frame_model(&mut controller);

    let labels = projected
        .tag_sidebar
        .option_pills
        .iter()
        .map(|pill| pill.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["Texture", "Rare FX"]);
    assert_eq!(
        projected.tag_sidebar.option_pills[0].state,
        BrowserTagState::On
    );
    assert!(projected.tag_sidebar.create_pill.is_none());
}

#[test]
fn browser_projection_sidebar_filters_candidates_and_exposes_create_state() {
    let (mut controller, source) = browser_projection_controller_with_source(vec![
        browser_projection_test_entry("first.wav"),
        browser_projection_test_entry("second.wav"),
    ]);
    let db = controller.database_for(&source).unwrap();
    db.assign_tag_to_path(std::path::Path::new("first.wav"), "Deep Kick")
        .unwrap();
    db.assign_tag_to_path(std::path::Path::new("second.wav"), "Deep Kick")
        .unwrap();
    db.assign_tag_to_path(std::path::Path::new("first.wav"), "Texture")
        .unwrap();
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.selection.selected_paths = vec![
        std::path::PathBuf::from("first.wav"),
        std::path::PathBuf::from("second.wav"),
    ];
    controller.mark_browser_selected_paths_changed();
    controller.ui.browser.tag_sidebar_input = String::from("kick");

    let projected = project_browser_panel_frame_model(&mut controller);

    assert_eq!(projected.tag_sidebar.option_pills.len(), 1);
    assert_eq!(
        projected.tag_sidebar.option_pills[0].label.as_str(),
        "Deep Kick"
    );
    assert_eq!(
        projected.tag_sidebar.option_pills[0].state,
        BrowserTagState::On
    );
    assert!(projected.tag_sidebar.create_pill.is_none());
    controller.ui.browser.tag_sidebar_input = String::from("vinyl crackle");
    let projected = project_browser_panel_frame_model(&mut controller);
    assert_eq!(
        projected
            .tag_sidebar
            .create_pill
            .as_ref()
            .map(|pill| pill.label.as_str()),
        Some("Create \"vinyl crackle\"")
    );
}

#[test]
fn browser_projection_sidebar_projects_mixed_normal_tag_state() {
    let (mut controller, source) = browser_projection_controller_with_source(vec![
        browser_projection_test_entry("first.wav"),
        browser_projection_test_entry("second.wav"),
    ]);
    controller
        .database_for(&source)
        .unwrap()
        .assign_tag_to_path(std::path::Path::new("first.wav"), "Texture")
        .unwrap();
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.selection.selected_paths = vec![
        std::path::PathBuf::from("first.wav"),
        std::path::PathBuf::from("second.wav"),
    ];
    controller.mark_browser_selected_paths_changed();

    let projected = project_browser_panel_frame_model(&mut controller);

    assert_eq!(
        projected.tag_sidebar.option_pills[0].state,
        BrowserTagState::Mixed
    );
}

/// Browser projection should expose manual viewport state for native scrollbar rendering.
#[test]
fn browser_projection_exposes_manual_viewport_state() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.selection.autoscroll = false;
    controller.ui.browser.viewport.view_window_start = 1_470;
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 1_506 };

    let projected = project_browser_panel_frame_model(&mut controller);

    assert!(!projected.autoscroll);
    assert_eq!(projected.view_start_row, 1_470);
    assert_eq!(projected.visible_count, 1_506);
}

fn browser_projection_controller_with_source(
    entries: Vec<crate::sample_sources::WavEntry>,
) -> (AppController, crate::sample_sources::SampleSource) {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.keep().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(entries);
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 2 };
    (controller, source)
}

/// Browser projection should prefer the selected browser sample label over stale loaded state.
#[test]
fn browser_projection_prefers_selected_sample_label_when_loaded_state_is_stale() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.selection.last_focused_path =
        Some(std::path::PathBuf::from("browser/current_take.wav"));
    controller.ui.loaded_wav = Some(std::path::PathBuf::from("waveform/stale_loaded_take.wav"));

    let projected = project_browser_panel_frame_model(&mut controller);

    assert_eq!(
        projected.focused_sample_label.as_deref(),
        Some("current_take")
    );
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

/// Browser row projection should surface normalized similarity strengths during similarity search.
#[test]
fn browser_projection_attaches_similarity_strength_for_similarity_query_rows() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.set_wav_entries_for_tests(vec![
        browser_projection_test_entry("anchor.wav"),
        browser_projection_test_entry("close.wav"),
        browser_projection_test_entry("far.wav"),
    ]);
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 3 };
    controller.ui.browser.search.similar_query =
        Some(crate::app_core::app_api::state::SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor.wav"),
            indices: vec![0, 1, 2],
            scores: vec![1.0, 0.5, -1.0],
            anchor_index: Some(0),
        });

    let projected = project_browser_model(&mut controller);

    assert_eq!(projected.rows.len(), 3);
    assert_eq!(projected.rows[0].similarity_display_strength, Some(255));
    assert!(
        projected.rows[1]
            .similarity_display_strength
            .zip(projected.rows[2].similarity_display_strength)
            .is_some_and(|(middle, weakest)| middle > weakest)
    );
    assert_eq!(projected.rows[2].similarity_display_strength, Some(0));
}

/// Browser row projection should leave similarity strength empty outside similarity search mode.
#[test]
fn browser_projection_omits_similarity_strength_without_similarity_query() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.set_wav_entries_for_tests(vec![
        browser_projection_test_entry("anchor.wav"),
        browser_projection_test_entry("close.wav"),
    ]);
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 2 };

    let projected = project_browser_model(&mut controller);

    assert!(
        projected
            .rows
            .iter()
            .all(|row| row.similarity_display_strength.is_none())
    );
}

/// Duplicate cleanup mode should keep the compact similarity bar disabled.
#[test]
fn browser_projection_omits_similarity_strength_during_duplicate_cleanup() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    let source_id = crate::sample_sources::SourceId::new();
    controller.select_browser_source_for_tests(source_id.clone());
    controller.set_wav_entries_for_tests(vec![
        browser_projection_test_entry("anchor.wav"),
        browser_projection_test_entry("close.wav"),
    ]);
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 2 };
    controller.ui.browser.search.similar_query =
        Some(crate::app_core::app_api::state::SimilarQuery {
            sample_id: String::from("sample-id"),
            label: String::from("anchor.wav"),
            indices: vec![0, 1],
            scores: vec![1.0, 0.4],
            anchor_index: Some(0),
        });
    controller.ui.browser.duplicate_cleanup = Some(
        crate::app_core::app_api::state::BrowserDuplicateCleanupState::new(
            source_id,
            String::from("sample-id"),
            std::path::PathBuf::from("anchor.wav"),
            String::from("anchor.wav"),
            vec![0, 1],
            vec![1.0, 0.4],
            0,
        ),
    );

    let projected = project_browser_model(&mut controller);

    assert!(
        projected
            .rows
            .iter()
            .all(|row| row.similarity_display_strength.is_none())
    );
}
