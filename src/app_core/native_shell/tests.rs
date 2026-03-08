use super::*;
use crate::app_core::app_api::state::SampleBrowserIndex;

/// Selected-column projection should default to the neutral middle column when nothing is focused.
#[test]
fn selected_column_defaults_to_middle_column_without_selection() {
    let ui = UiState::default();
    assert_eq!(selected_column_index(&ui), 1);
}

/// Browser render windows should cap to the configured maximum when no focus hints exist.
#[test]
fn browser_render_window_limits_to_target_size() {
    let (start, len) = browser_render_window(500, None, None, 0);
    assert_eq!(start, 0);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
}

/// Browser render windows should keep the current window stable for interior focus changes.
#[test]
fn browser_render_window_keeps_existing_window_for_interior_focus_changes() {
    let (start, len) = browser_render_window(500, Some(250), None, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 200);
}

/// Browser render windows should nudge downward when focus enters the bottom guard band.
#[test]
fn browser_render_window_scrolls_when_focus_reaches_bottom_guard_band() {
    let (start, len) = browser_render_window(500, Some(453), None, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 201);
}

/// Browser render windows should nudge upward when focus enters the top guard band.
#[test]
fn browser_render_window_scrolls_when_focus_reaches_top_guard_band() {
    let (start, len) = browser_render_window(500, Some(202), None, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 199);
}

/// Browser render windows should keep the fourth row from the top stable.
#[test]
fn browser_render_window_keeps_fourth_row_from_top_stable() {
    let (start, len) = browser_render_window(500, Some(203), None, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 200);
}

/// Browser render windows should keep the fourth row from the bottom stable.
#[test]
fn browser_render_window_keeps_fourth_row_from_bottom_stable() {
    let (start, len) = browser_render_window(500, Some(452), None, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 200);
}

/// Browser render windows should clamp near the end instead of overrunning visible rows.
#[test]
fn browser_render_window_clamps_near_end_of_visible_rows() {
    let (start, len) = browser_render_window(500, Some(490), None, 200);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 238);
}

/// Browser render windows should still honor the hard row cap for very large datasets.
#[test]
fn browser_render_window_limits_large_visible_sets_to_cap() {
    let (start, len) = browser_render_window(1_200, None, None, 0);
    assert_eq!(start, 0);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
}

/// Browser render windows should keep interior selections stable and still clamp correctly at the tail.
#[test]
fn browser_render_window_keeps_stable_window_and_tail_clamps_for_large_visible_sets() {
    let (center_start, center_len) = browser_render_window(1_200, Some(800), None, 700);
    assert_eq!(center_len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(center_start, 700);

    let (tail_start, tail_len) = browser_render_window(1_200, Some(1_190), None, 700);
    assert_eq!(tail_len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(tail_start, 938);
}

/// Browser render windows should still clamp at the hard tail when the focus reaches the last row.
#[test]
fn browser_render_window_clamps_at_tail_for_last_visible_row() {
    let (start, len) = browser_render_window(1_200, Some(1_199), None, 700);
    assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    assert_eq!(start, 944);
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
    controller.ui.browser.sort = SampleBrowserSort::PlaybackAgeDesc;
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
    controller.ui.browser.visible = crate::app_core::app_api::state::VisibleRows::All { total: 42 };
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
    controller.ui.browser.search_focus_requested = true;
    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.search_placeholder.as_deref(), Some("▌"));
}

/// Browser chrome projection should expose the toolbar copy shown in the native shell.
#[test]
fn browser_chrome_projection_exposes_toolbar_and_tab_copy() {
    let mut ui = UiState::default();
    ui.browser.sort = SampleBrowserSort::Similarity;
    ui.browser.similarity_sort_follow_loaded = true;
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
    ui.browser.search_focus_requested = true;
    let projected = project_browser_chrome_model(&ui, 7);
    assert_eq!(projected.search_prefix_label, "Search • focused");
    assert_eq!(projected.search_placeholder, "▌");
}

/// Waveform projection should derive tempo and zoom labels from UI waveform state.
#[test]
fn waveform_projection_exposes_tempo_and_zoom_labels() {
    let mut ui = UiState::default();
    ui.waveform.bpm_value = Some(128.0);
    ui.waveform.view.start = 0.25;
    ui.waveform.view.end = 0.75;
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.bpm_value = Some(128.0);
    controller.ui.waveform.view.start = 0.25;
    controller.ui.waveform.view.end = 0.75;
    let projected = project_waveform_model(&mut controller);
    assert_eq!(projected.tempo_label.as_deref(), Some("128.0 BPM"));
    assert_eq!(projected.zoom_label.as_deref(), Some("200%"));
    assert!(projected.waveform_image.is_none());
}

/// Waveform projection should pass through raster payload bytes unchanged when present.
#[test]
fn waveform_projection_passes_raster_image_payload() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.image = Some(crate::waveform::WaveformImage {
        size: [2, 1],
        pixels: vec![
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(10, 20, 30, 40),
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(11, 21, 31, 41),
        ],
    });
    let projected = project_waveform_model(&mut controller);
    let waveform_image = projected
        .waveform_image
        .as_ref()
        .expect("waveform image should be projected");
    assert_eq!(waveform_image.width, 2);
    assert_eq!(waveform_image.height, 1);
    assert_eq!(
        waveform_image.pixels.as_ref(),
        &[10, 20, 30, 40, 11, 21, 31, 41]
    );
}

#[test]
/// Waveform projection should expose edit fade handle positions when fades are configured.
fn waveform_projection_includes_edit_fade_handles() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.edit_selection = Some(
        crate::selection::SelectionRange::new(0.2, 0.8)
            .with_fade_in(0.25, 0.75)
            .with_fade_in_mute(0.1)
            .with_fade_out(0.5, 0.25)
            .with_fade_out_mute(0.2),
    );

    let projected = project_waveform_model(&mut controller);
    assert_eq!(
        projected.edit_selection_milli,
        Some(NormalizedRangeModel::new(200, 800))
    );
    assert_eq!(projected.edit_fade_in_end_milli, Some(350));
    assert_eq!(projected.edit_fade_in_mute_start_milli, Some(140));
    assert_eq!(projected.edit_fade_in_curve_milli, Some(750));
    assert_eq!(projected.edit_fade_out_start_milli, Some(500));
    assert_eq!(projected.edit_fade_out_mute_end_milli, Some(920));
    assert_eq!(projected.edit_fade_out_curve_milli, Some(250));
}

/// Build a controller fixture with non-default fields for full app-model parity checks.
fn app_model_projection_fixture_controller() -> AppController {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.status.text = String::from("Projection fixture status");
    controller.ui.volume = 1.25;
    controller.ui.browser.visible = crate::app_core::app_api::state::VisibleRows::All { total: 24 };
    controller.ui.browser.sort = SampleBrowserSort::PlaybackAgeAsc;
    controller.ui.browser.search_query = String::from("kick");
    controller.ui.browser.search_busy = true;
    controller.ui.browser.selected = Some(SampleBrowserIndex {
        column: TriageFlagColumn::Keep,
        row: 0,
    });
    controller.ui.browser.active_tab = SampleBrowserTab::List;
    controller.ui.waveform.loop_enabled = true;
    controller.ui.update.status = UpdateStatus::Checking;
    controller
}

#[test]
/// Staged projection helpers should assemble the same app model as `project_app_model`.
fn project_app_model_matches_staged_projection_helpers() {
    let mut expected_controller = app_model_projection_fixture_controller();
    let derived_inputs = derive_project_app_model_inputs(&expected_controller);
    let core_models = materialize_project_app_model_core(&mut expected_controller, &derived_inputs);
    let overlay_and_chrome_models = materialize_project_app_model_overlay_and_chrome(
        &expected_controller.ui,
        core_models.browser.visible_count,
    );
    let expected =
        assemble_project_app_model(derived_inputs, core_models, overlay_and_chrome_models);

    let mut actual_controller = app_model_projection_fixture_controller();
    let actual = project_app_model(&mut actual_controller);

    assert_eq!(actual, expected);
}

/// Waveform chrome projection should mirror loop/channel/toggle state into native labels.
#[test]
fn waveform_chrome_projection_reflects_loop_hint() {
    let mut ui = UiState::default();
    ui.waveform.loop_enabled = false;
    ui.waveform.channel_view = crate::waveform::WaveformChannelView::Mono;
    let projected = project_waveform_chrome_model(&ui);
    assert_eq!(projected.transport_hint, "Loop disabled");
    assert_eq!(
        projected.channel_view,
        radiant::app::WaveformChannelViewModel::Mono
    );
    assert!(!projected.normalized_audition_enabled);
    assert!(!projected.bpm_snap_enabled);
    assert!(!projected.transient_snap_enabled);
    assert!(projected.transient_markers_enabled);
    assert!(!projected.slice_mode_enabled);

    ui.waveform.loop_enabled = true;
    ui.waveform.channel_view = crate::waveform::WaveformChannelView::SplitStereo;
    ui.waveform.normalized_audition_enabled = true;
    ui.waveform.bpm_snap_enabled = true;
    ui.waveform.transient_snap_enabled = true;
    ui.waveform.transient_markers_enabled = false;
    ui.waveform.slice_mode_enabled = true;
    let projected = project_waveform_chrome_model(&ui);
    assert_eq!(projected.transport_hint, "Loop enabled");
    assert_eq!(
        projected.channel_view,
        radiant::app::WaveformChannelViewModel::Stereo
    );
    assert!(projected.normalized_audition_enabled);
    assert!(projected.bpm_snap_enabled);
    assert!(projected.transient_snap_enabled);
    assert!(!projected.transient_markers_enabled);
    assert!(projected.slice_mode_enabled);
}

/// Update projection should expose the status text and action hints for each update state.
#[test]
fn update_projection_exposes_status_and_action_hint_labels() {
    let mut ui = UiState::default();
    let projected = project_update_model(&ui);
    assert_eq!(projected.status, UpdateStatusModel::Idle);
    assert_eq!(projected.status_label, "Updates: idle");
    assert_eq!(projected.action_hint_label, "Action: check");
    assert!(projected.release_notes_label.is_empty());

    ui.update.status = UpdateStatus::UpdateAvailable;
    ui.update.available_tag = Some(String::from("v20.1.0"));
    ui.update.available_url = Some(String::from("https://example.invalid/release"));
    ui.update.available_published_at = Some(String::from("2026-02-01T12:00:00Z"));
    let projected = project_update_model(&ui);
    assert_eq!(projected.status, UpdateStatusModel::Available);
    assert_eq!(
        projected.status_label,
        "Update available: v20.1.0 (manual install required)"
    );
    assert_eq!(
        projected.action_hint_label,
        "Actions: open | install(manual) | dismiss"
    );
    assert_eq!(
        projected.release_notes_label,
        "Release: v20.1.0 (2026-02-01T12:00:00Z) | Signed manual install required"
    );

    ui.update.status = UpdateStatus::Error;
    ui.update.last_error = Some(String::from("network timeout"));
    let projected = project_update_model(&ui);
    assert_eq!(projected.status, UpdateStatusModel::Error);
    assert_eq!(
        projected.status_label,
        "Update check failed: network timeout"
    );
    assert_eq!(projected.action_hint_label, "Action: retry");
    assert!(projected.release_notes_label.is_empty());
}

/// Map projection should expose legend, selection, hover, cluster, and viewport summary text.
#[test]
fn map_projection_exposes_legend_selection_and_viewport_labels() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
    controller.ui.map.zoom = 1.75;
    controller.ui.map.pan.x = 12.0;
    controller.ui.map.pan.y = -8.0;
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
        sample_id: std::sync::Arc::<str>::from("source::kick_24.wav"),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }];
    controller.ui.map.cached_points_source_id = None;
    controller.ui.map.cached_points_umap_version = Some(String::from("v1"));
    controller.ui.map.cached_points_revision = 7;
    controller.ui.map.selected_sample_id = Some(String::from("source::kick_24.wav"));
    controller.ui.map.hovered_sample_id = Some(String::from("source::kick_hover.wav"));

    let projected = project_map_model(&mut controller);
    assert!(projected.active);
    assert!(projected.legend_label.starts_with("Render:"));
    assert!(projected.selection_label.contains("Selection:"));
    assert_eq!(
        projected.selected_sample_id.as_deref(),
        Some("source::kick_24.wav")
    );
    assert!(projected.hover_label.contains("Hover:"));
    assert!(projected.cluster_label.starts_with("Clusters:"));
    assert_eq!(projected.viewport_label, "zoom 1.75x | pan (12, -8)");
}

/// Map projection should reuse cached points when the current query key still matches them.
#[test]
fn map_projection_uses_cached_points_when_query_key_matches() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
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
        sample_id: std::sync::Arc::<str>::from("source::kick.wav"),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }];
    controller.ui.map.cached_points_source_id = None;
    controller.ui.map.cached_points_umap_version = Some(String::from("v1"));
    controller.ui.map.cached_points_revision = 7;

    let projected = project_map_model(&mut controller);
    assert!(projected.active);
    assert_eq!(projected.error, None);
    assert_eq!(projected.summary, "1 points");
    assert_eq!(projected.points.len(), 1);
    assert_eq!(controller.ui.map.cached_points_revision, 7);
}

#[test]
/// Normalized point cache should be reused while map point revision is unchanged.
fn map_projection_reuses_normalized_points_when_revision_is_unchanged() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
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
        sample_id: std::sync::Arc::<str>::from("source::kick.wav"),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }];
    controller.ui.map.cached_points_source_id = None;
    controller.ui.map.cached_points_umap_version = Some(String::from("v1"));
    controller.ui.map.cached_points_revision = 7;

    let first = project_map_model(&mut controller);
    controller.ui.map.cached_points[0].x = 1.0;
    controller.ui.map.cached_points[0].y = 1.0;
    let second = project_map_model(&mut controller);

    assert_eq!(first.points.len(), 1);
    assert_eq!(second.points.len(), 1);
    assert_eq!(first.points[0].x_milli, second.points[0].x_milli);
    assert_eq!(first.points[0].y_milli, second.points[0].y_milli);
    assert!(std::sync::Arc::ptr_eq(&first.points, &second.points));
    assert!(controller.projected_map_points_key.is_some());
}

#[test]
/// Map projection should reuse retained point payloads when only selection/focus changes.
fn map_projection_reuses_retained_points_for_selection_overlay_changes() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
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
        sample_id: std::sync::Arc::<str>::from("source::kick.wav"),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }];
    controller.ui.map.cached_points_source_id = None;
    controller.ui.map.cached_points_umap_version = Some(String::from("v1"));
    controller.ui.map.cached_points_revision = 7;

    let first = project_map_model(&mut controller);
    controller.ui.map.selected_sample_id = Some(String::from("source::kick.wav"));
    let second = project_map_model(&mut controller);

    assert!(std::sync::Arc::ptr_eq(&first.points, &second.points));
    assert_eq!(
        second.selected_sample_id.as_deref(),
        Some("source::kick.wav")
    );
}

#[test]
/// Changing cached-point revision should force normalized cache rebuild.
fn map_projection_rebuilds_normalized_points_after_revision_change() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
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
        sample_id: std::sync::Arc::<str>::from("source::kick.wav"),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }];
    controller.ui.map.cached_points_source_id = None;
    controller.ui.map.cached_points_umap_version = Some(String::from("v1"));
    controller.ui.map.cached_points_revision = 7;

    let first = project_map_model(&mut controller);
    controller.ui.map.cached_points[0].x = 1.0;
    controller.ui.map.cached_points[0].y = 1.0;
    controller.ui.map.cached_points_revision =
        controller.ui.map.cached_points_revision.saturating_add(1);
    let second = project_map_model(&mut controller);

    assert_eq!(first.points.len(), 1);
    assert_eq!(second.points.len(), 1);
    assert_ne!(first.points[0].x_milli, second.points[0].x_milli);
    assert_ne!(first.points[0].y_milli, second.points[0].y_milli);
    assert!(!std::sync::Arc::ptr_eq(&first.points, &second.points));
    assert_eq!(second.points[0].x_milli, 1000);
    assert_eq!(second.points[0].y_milli, 1000);
}

/// Map projection should discard cached normalized points when the UMAP version changes.
#[test]
fn map_projection_does_not_reuse_stale_cache_after_umap_version_change() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
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
        sample_id: std::sync::Arc::<str>::from("source::kick.wav"),
        x: 0.0,
        y: 0.0,
        cluster_id: Some(1),
    }];
    controller.ui.map.cached_points_source_id = None;
    controller.ui.map.cached_points_umap_version = Some(String::from("v1"));
    controller.ui.map.umap_version = String::from("v2");

    let projected = project_map_model(&mut controller);
    assert!(projected.active);
    assert!(projected.error.is_some());
    assert!(projected.points.is_empty());
}

/// Browser action availability should stay disabled until focus or selection exists.
#[test]
fn browser_actions_require_focus_or_selection() {
    let mut ui = UiState::default();
    let projected = project_browser_actions_model(&ui);
    assert!(!projected.can_rename);
    assert!(!projected.can_delete);
    assert!(!projected.can_tag);

    ui.browser.selected_visible = Some(0);
    let projected = project_browser_actions_model(&ui);
    assert!(projected.can_rename);
    assert!(projected.can_delete);
    assert!(projected.can_tag);
}

/// Browser rename prompts should win over destructive waveform prompts when both are present.
#[test]
fn confirm_prompt_prefers_browser_rename_when_multiple_prompts_exist() {
    let mut ui = UiState::default();
    ui.browser.pending_action = Some(SampleBrowserActionPrompt::Rename {
        target: std::path::PathBuf::from("kick.wav"),
        name: String::from("kick"),
    });
    ui.waveform.pending_destructive = Some(DestructiveEditPrompt {
        edit: DestructiveSelectionEdit::TrimSelection,
        title: String::from("Trim selection"),
        message: String::from("Apply trim?"),
    });
    let projected = project_confirm_prompt_model(&ui);
    assert!(projected.visible);
    assert_eq!(projected.kind, Some(ConfirmPromptKind::BrowserRename));
}

/// Inline folder creation state should project into the shared confirm prompt model.
#[test]
fn confirm_prompt_projects_folder_create_inline_state() {
    let mut ui = UiState::default();
    ui.sources.folders.new_folder = Some(InlineFolderCreation {
        parent: std::path::PathBuf::from("drums"),
        name: String::from("kicks"),
        focus_requested: true,
    });
    let projected = project_confirm_prompt_model(&ui);
    assert!(projected.visible);
    assert_eq!(projected.kind, Some(ConfirmPromptKind::FolderCreate));
    assert_eq!(projected.confirm_label, "Create");
    assert_eq!(projected.input_value.as_deref(), Some("kicks"));
    assert_eq!(
        projected.input_placeholder.as_deref(),
        Some("New folder name")
    );
}

/// Folder-create projection should surface duplicate-name and separator validation errors.
#[test]
fn confirm_prompt_projects_folder_create_validation_errors() {
    let mut ui = UiState::default();
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums/existing"),
        name: String::from("existing"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.new_folder = Some(InlineFolderCreation {
        parent: std::path::PathBuf::from("drums"),
        name: String::from("existing"),
        focus_requested: true,
    });
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
        Some("Folder already exists: drums/existing")
    );

    if let Some(new_folder) = ui.sources.folders.new_folder.as_mut() {
        new_folder.name = String::from("bad/name");
    }
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
        Some("Folder name cannot contain path separators")
    );
}

/// Folder-rename projection should surface duplicate-name and separator validation errors.
#[test]
fn confirm_prompt_projects_folder_rename_validation_errors() {
    let mut ui = UiState::default();
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums"),
        name: String::from("drums"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: true,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("kicks"),
        name: String::from("kicks"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.pending_action = Some(FolderActionPrompt::Rename {
        target: std::path::PathBuf::from("drums"),
        name: String::from("kicks"),
    });
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
        Some("Folder already exists: kicks")
    );

    ui.sources.folders.pending_action = Some(FolderActionPrompt::Rename {
        target: std::path::PathBuf::from("drums"),
        name: String::from("../bad"),
    });
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
        Some("Folder name cannot contain path separators")
    );
}

/// Progress overlay projection should preserve modal and cancel-requested flags.
#[test]
fn progress_overlay_projection_preserves_cancel_state() {
    let mut ui = UiState::default();
    ui.progress.visible = true;
    ui.progress.modal = true;
    ui.progress.title = String::from("Scanning");
    ui.progress.completed = 3;
    ui.progress.total = 9;
    ui.progress.cancelable = true;
    ui.progress.cancel_requested = true;
    let projected = project_progress_overlay_model(&ui);
    assert!(projected.visible);
    assert!(projected.modal);
    assert!(projected.cancelable);
    assert!(projected.cancel_requested);
    assert_eq!(projected.completed, 3);
    assert_eq!(projected.total, 9);
}

/// Destructive folder actions should require focus on a non-root folder row.
#[test]
fn folder_actions_require_non_root_focus_for_destructive_actions() {
    let mut ui = UiState::default();
    ui.sources.selected = Some(0);
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::new(),
        name: String::from("Root"),
        depth: 0,
        has_children: true,
        expanded: true,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: true,
        root_filter_mode: None,
    });
    ui.sources.folders.focused = Some(0);
    let projected = project_sources_model(&ui);
    assert!(projected.folder_actions.can_create_folder);
    assert!(projected.folder_actions.can_create_folder_at_root);
    assert!(!projected.folder_actions.can_rename_folder);
    assert!(!projected.folder_actions.can_delete_folder);

    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums"),
        name: String::from("drums"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: true,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.focused = Some(1);
    let projected = project_sources_model(&ui);
    assert!(projected.folder_actions.can_rename_folder);
    assert!(projected.folder_actions.can_delete_folder);
}

/// Root folder creation should remain available even when there are no source rows yet.
#[test]
fn folder_actions_allow_root_creation_when_no_sources_exist() {
    let ui = UiState::default();
    let projected = project_sources_model(&ui);
    assert!(!projected.folder_actions.can_create_folder);
    assert!(projected.folder_actions.can_create_folder_at_root);
}

/// Recovery log clearing should stay disabled while delete recovery work is still running.
#[test]
fn folder_actions_disable_recovery_clear_while_recovery_is_running() {
    let mut ui = UiState::default();
    ui.sources
        .folders
        .delete_recovery
        .entries
        .push(FolderDeleteRecoveryEntry {
            source_label: String::from("source"),
            relative_path: std::path::PathBuf::from("drums"),
            action: FolderDeleteRecoveryAction::Restore,
            status: FolderDeleteRecoveryStatus::Completed,
            detail: None,
        });
    ui.sources.folders.delete_recovery.in_progress = true;
    let projected = project_sources_model(&ui);
    assert!(!projected.folder_actions.can_clear_recovery_log);
}

#[test]
/// Retained browser row cache should survive visible-row revision changes for the same source.
fn browser_row_cache_persists_when_visible_revision_changes() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    let source_id = crate::sample_sources::SourceId::new();
    controller.select_browser_source_for_tests(source_id.clone());
    controller.projected_browser_rows_source_id = Some(source_id);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            bucket_label: String::new(),
            missing: false,
            looped: false,
            bpm_value_bits: None,
            long_sample_mark: false,
        },
    );
    controller.ui.browser.visible_rows_revision = 8;

    refresh_projected_browser_row_cache(&mut controller);

    assert!(controller.projected_browser_rows.contains_key(&0));
}

#[test]
/// Retained browser row cache should clear when the selected source changes.
fn browser_row_cache_clears_when_selected_source_changes() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.projected_browser_rows_source_id = Some(crate::sample_sources::SourceId::new());
    controller.select_browser_source_for_tests(crate::sample_sources::SourceId::new());
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            bucket_label: String::new(),
            missing: false,
            looped: false,
            bpm_value_bits: None,
            long_sample_mark: false,
        },
    );

    refresh_projected_browser_row_cache(&mut controller);

    assert!(controller.projected_browser_rows.is_empty());
}

#[test]
/// Selected-path lookup cache should refresh when path content changes at equal length.
fn selected_path_lookup_refreshes_for_same_len_path_changes() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("first.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-a")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            missing: false,
            last_played_at: None,
        },
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("second.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-b")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            missing: false,
            last_played_at: None,
        },
    ]);
    controller.ui.browser.selected_paths = vec![std::path::PathBuf::from("first.wav")];
    controller.mark_browser_selected_paths_changed();
    refresh_projected_selected_paths_lookup(&mut controller);
    assert!(matches!(
        controller.projected_selected_paths_lookup,
        Some(crate::app_core::controller::ProjectedSelectedPathsLookup::Single(0))
    ));
    assert!(selected_index_is_selected(&controller, 0));
    assert!(!selected_index_is_selected(&controller, 1));

    controller.ui.browser.selected_paths = vec![std::path::PathBuf::from("second.wav")];
    controller.mark_browser_selected_paths_changed();
    refresh_projected_selected_paths_lookup(&mut controller);
    assert!(matches!(
        controller.projected_selected_paths_lookup,
        Some(crate::app_core::controller::ProjectedSelectedPathsLookup::Single(1))
    ));
    assert!(!selected_index_is_selected(&controller, 0));
    assert!(selected_index_is_selected(&controller, 1));
}

#[test]
/// Cached browser rows should rebuild when stored tag/column metadata is stale.
fn cached_browser_row_rebuilds_when_stored_tag_column_is_stale() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::KEEP_1,
        looped: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            bucket_label: String::new(),
            missing: false,
            looped: false,
            bpm_value_bits: None,
            long_sample_mark: false,
        },
    );

    let Some(cached) = project_cached_browser_row(&mut controller, 0) else {
        panic!("cached row should exist");
    };

    assert_eq!(cached.0.column_index, 2);
    assert_eq!(cached.0.rating_level, 1);
}

#[test]
/// Cached browser rows should rebuild when stored missing metadata is stale.
fn cached_browser_row_rebuilds_when_stored_missing_state_is_stale() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        missing: true,
        last_played_at: None,
    }]);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            bucket_label: String::new(),
            missing: false,
            looped: false,
            bpm_value_bits: None,
            long_sample_mark: false,
        },
    );

    let Some(cached) = project_cached_browser_row(&mut controller, 0) else {
        panic!("cached row should exist");
    };

    assert!(cached.0.missing);
}

#[test]
/// Reusing the projection buffer should preserve the existing allocation.
fn browser_rows_projection_reuses_provided_buffer_capacity() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("snare.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.ui.browser.visible =
        crate::app_core::app_api::state::VisibleRows::List(vec![0usize].into());
    let mut rows = Vec::new();

    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);
    let first_capacity = rows.capacity();
    let first_ptr = rows.as_ptr();

    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows.capacity(), first_capacity);
    assert_eq!(rows.as_ptr(), first_ptr);
}

#[test]
/// BPM preload ranges should only include rows newly entering an unchanged browser window.
fn browser_bpm_preload_ranges_only_include_window_delta() {
    let source_id = crate::sample_sources::SourceId::new();
    let previous = ProjectedBrowserPreloadWindow {
        source_id: Some(source_id.clone()),
        visible_rows_revision: 11,
        window_start: 10,
        window_len: 5,
    };

    let ranges = browser_bpm_preload_ranges(Some(&previous), Some(&source_id), 11, 12, 5);

    assert_eq!(ranges, vec![(15, 2)]);
}

#[test]
fn status_bar_right_text_shows_column() {
    assert_eq!(status_bar_right_text(0), "col: 1/3");
}

/// Status-bar column text should remain stable for repeated equivalent inputs.
#[test]
fn status_bar_right_text_is_stable_across_input() {
    assert_eq!(status_bar_right_text(2), "col: 3/3");
}

#[test]
/// Motion projection should derive right-status text directly from selected column.
fn motion_projection_sets_status_right_from_selected_column() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.status.text = "x".repeat(8_192);
    controller.ui.browser.selected = Some(SampleBrowserIndex {
        column: TriageFlagColumn::Keep,
        row: 0,
    });

    let motion = project_motion_model(&mut controller);

    assert_eq!(motion.status_right, "col: 3/3");
}
