use super::*;

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
