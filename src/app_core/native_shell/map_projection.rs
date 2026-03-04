//! Map-panel projection and retained map-point cache helpers.

use super::waveform_projection::normalized_to_milli;
use super::*;
use std::path::Path;

pub(crate) fn project_map_model(controller: &mut AppController) -> MapPanelModel {
    let active = matches!(
        SampleBrowserTab::from(controller.ui.browser.active_tab),
        SampleBrowserTab::Map
    );
    let render_mode = match MapRenderMode::from(controller.ui.map.last_render_mode) {
        MapRenderMode::Heatmap => MapRenderModeModel::Heatmap,
        MapRenderMode::Points => MapRenderModeModel::Points,
    };
    let render_mode_label = match render_mode {
        MapRenderModeModel::Heatmap => "heatmap",
        MapRenderModeModel::Points => "points",
    };
    if !active {
        return MapPanelModel {
            active: false,
            summary: String::from("Map hidden"),
            legend_label: format!("Render: {render_mode_label}"),
            selection_label: String::from("Selection: —"),
            hover_label: String::from("Hover: —"),
            cluster_label: String::from("Clusters: —"),
            viewport_label: String::from("zoom 1.00x | pan (0, 0)"),
            error: None,
            render_mode,
            points: Vec::new(),
        };
    }

    let source_id = controller.current_source().map(|source| source.id);
    let source_id_key = source_id.as_ref().map(|id| id.as_str().to_string());
    let umap_version = controller.ui.map.umap_version.clone();
    let has_matching_bounds_cache = controller.ui.map.cached_bounds_source_id == source_id_key
        && controller.ui.map.cached_bounds_umap_version.as_deref() == Some(umap_version.as_str());
    let bounds = if has_matching_bounds_cache {
        controller.ui.map.bounds
    } else {
        match controller.umap_bounds(SIMILARITY_MODEL_ID, &umap_version, source_id.as_ref()) {
            Ok(bounds) => {
                let mapped_bounds = bounds.map(|value| MapBounds {
                    min_x: value.min_x,
                    max_x: value.max_x,
                    min_y: value.min_y,
                    max_y: value.max_y,
                });
                controller.ui.map.cached_bounds_source_id = source_id_key.clone();
                controller.ui.map.cached_bounds_umap_version = Some(umap_version.clone());
                controller.ui.map.bounds = mapped_bounds;
                controller.mark_map_dataset_projection_revision_dirty();
                mapped_bounds
            }
            Err(err) => {
                return MapPanelModel {
                    active: true,
                    summary: String::from("Map unavailable"),
                    legend_label: format!("Render: {render_mode_label}"),
                    selection_label: String::from("Selection: unavailable"),
                    hover_label: String::from("Hover: unavailable"),
                    cluster_label: String::from("Clusters: unavailable"),
                    viewport_label: format!(
                        "zoom {:.2}x | pan ({:.0}, {:.0})",
                        controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
                    ),
                    error: Some(err),
                    render_mode,
                    points: Vec::new(),
                };
            }
        }
    };
    let Some(bounds) = bounds else {
        return MapPanelModel {
            active: true,
            summary: String::from("No map data (run similarity prep)"),
            legend_label: format!("Render: {render_mode_label}"),
            selection_label: String::from("Selection: —"),
            hover_label: String::from("Hover: —"),
            cluster_label: String::from("Clusters: —"),
            viewport_label: format!(
                "zoom {:.2}x | pan ({:.0}, {:.0})",
                controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
            ),
            error: None,
            render_mode,
            points: Vec::new(),
        };
    };
    let query_bounds = MapQueryBounds {
        min_x: bounds.min_x,
        max_x: bounds.max_x,
        min_y: bounds.min_y,
        max_y: bounds.max_y,
    };
    let has_matching_points_cache = controller.ui.map.cached_points_source_id == source_id_key
        && controller.ui.map.cached_points_umap_version.as_deref() == Some(umap_version.as_str())
        && controller.ui.map.last_query == Some(query_bounds);
    if !has_matching_points_cache {
        match controller.umap_points_in_bounds(UmapPointQuery {
            model_id: SIMILARITY_MODEL_ID,
            umap_version: &umap_version,
            cluster_method: "umap",
            cluster_umap_version: &umap_version,
            source_id: source_id.as_ref(),
            bounds: query_bounds,
            limit: MAX_RENDERED_MAP_POINTS,
        }) {
            Ok(points) => {
                controller.ui.map.cached_points = points
                    .iter()
                    .map(|point| MapPoint {
                        sample_id: point.sample_id.clone(),
                        x: point.x,
                        y: point.y,
                        cluster_id: point.cluster_id,
                    })
                    .collect::<Vec<_>>();
                controller.ui.map.cached_points_source_id = source_id_key.clone();
                controller.ui.map.cached_points_umap_version = Some(umap_version.clone());
                controller.ui.map.last_query = Some(query_bounds);
                controller.ui.map.cached_points_revision =
                    controller.ui.map.cached_points_revision.saturating_add(1);
                controller.mark_map_dataset_projection_revision_dirty();
                controller.mark_map_query_projection_revision_dirty();
            }
            Err(err) => {
                return MapPanelModel {
                    active: true,
                    summary: String::from("Map query failed"),
                    legend_label: format!("Render: {render_mode_label}"),
                    selection_label: String::from("Selection: unavailable"),
                    hover_label: String::from("Hover: unavailable"),
                    cluster_label: String::from("Clusters: unavailable"),
                    viewport_label: format!(
                        "zoom {:.2}x | pan ({:.0}, {:.0})",
                        controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
                    ),
                    error: Some(err),
                    render_mode,
                    points: Vec::new(),
                };
            }
        }
    }

    let focused_sample_id = controller.selected_sample_id();
    let selected_sample_id = controller.ui.map.selected_sample_id.clone();
    let projection_key = map_projection_cache_key(
        source_id_key.as_deref(),
        umap_version.as_str(),
        controller.ui.map.cached_points_revision,
        query_bounds,
    );
    refresh_projected_map_points_cache(controller, projection_key, bounds);
    let cluster_count = controller.projected_map_cluster_count;
    let points = project_map_points_model(
        controller.projected_map_points.as_slice(),
        selected_sample_id.as_deref(),
        focused_sample_id.as_deref(),
    );
    let selection_label = controller
        .ui
        .map
        .selected_sample_id
        .as_deref()
        .map(short_sample_label)
        .map(|label| format!("Selection: {label}"))
        .or_else(|| {
            focused_sample_id
                .as_deref()
                .map(short_sample_label)
                .map(|label| format!("Focus: {label}"))
        })
        .unwrap_or_else(|| String::from("Selection: —"));
    let hover_label = controller
        .ui
        .map
        .hovered_sample_id
        .as_deref()
        .or(controller.ui.map.paint_hover_active_id.as_deref())
        .map(short_sample_label)
        .map(|label| format!("Hover: {label}"))
        .unwrap_or_else(|| String::from("Hover: —"));
    let cluster_label = if cluster_count == 0 {
        String::from("Clusters: —")
    } else {
        format!("Clusters: {cluster_count}")
    };
    let viewport_label = format!(
        "zoom {:.2}x | pan ({:.0}, {:.0})",
        controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
    );
    let summary = format!("{} points", points.len());
    MapPanelModel {
        active: true,
        summary,
        legend_label: format!("Render: {render_mode_label}"),
        selection_label,
        hover_label,
        cluster_label,
        viewport_label,
        error: None,
        render_mode,
        points,
    }
}

/// Build a retained map-projection cache key from active map source/query state.
fn map_projection_cache_key(
    source_id: Option<&str>,
    umap_version: &str,
    points_revision: u64,
    query_bounds: MapQueryBounds,
) -> ProjectedMapPointsCacheKey {
    ProjectedMapPointsCacheKey {
        source_id_hash: hash_scalar(source_id.unwrap_or_default()),
        umap_version_hash: hash_scalar(umap_version),
        points_revision,
        query_min_x_bits: query_bounds.min_x.to_bits(),
        query_max_x_bits: query_bounds.max_x.to_bits(),
        query_min_y_bits: query_bounds.min_y.to_bits(),
        query_max_y_bits: query_bounds.max_y.to_bits(),
    }
}

/// Refresh retained normalized map-point cache only when projection key changes.
fn refresh_projected_map_points_cache(
    controller: &mut AppController,
    key: ProjectedMapPointsCacheKey,
    bounds: MapBounds,
) {
    if controller.projected_map_points_key == Some(key) {
        return;
    }
    let (projected_points, cluster_count) = {
        let points = controller.ui.map.cached_points.as_slice();
        build_projected_map_points_cache(bounds, points)
    };
    controller.projected_map_points_key = Some(key);
    controller.projected_map_points = projected_points;
    controller.projected_map_cluster_count = cluster_count;
}

/// Build normalized map-point cache entries and unique cluster summary in one pass.
fn build_projected_map_points_cache(
    bounds: MapBounds,
    points: &[MapPoint],
) -> (Vec<ProjectedMapPointCacheEntry>, usize) {
    let denom_x = (bounds.max_x - bounds.min_x).max(1e-6);
    let denom_y = (bounds.max_y - bounds.min_y).max(1e-6);
    let mut cluster_ids = HashSet::new();
    let mut projected_points = Vec::with_capacity(points.len());
    for point in points {
        if let Some(cluster_id) = point.cluster_id {
            cluster_ids.insert(cluster_id);
        }
        let x = ((point.x - bounds.min_x) / denom_x).clamp(0.0, 1.0);
        let y = ((point.y - bounds.min_y) / denom_y).clamp(0.0, 1.0);
        projected_points.push(ProjectedMapPointCacheEntry {
            sample_id: point.sample_id.clone(),
            x_milli: normalized_to_milli(x),
            y_milli: normalized_to_milli(y),
            cluster_id: point.cluster_id,
        });
    }
    (projected_points, cluster_ids.len())
}

/// Project final map points by applying dynamic selected/focused state flags.
fn project_map_points_model(
    projected_points: &[ProjectedMapPointCacheEntry],
    selected_sample_id: Option<&str>,
    focused_sample_id: Option<&str>,
) -> Vec<MapPointModel> {
    let mut points = Vec::with_capacity(projected_points.len());
    for point in projected_points {
        points.push(MapPointModel {
            sample_id: point.sample_id.clone(),
            x_milli: point.x_milli,
            y_milli: point.y_milli,
            cluster_id: point.cluster_id,
            selected: selected_sample_id.is_some_and(|selected| selected == point.sample_id),
            focused: focused_sample_id.is_some_and(|focused| focused == point.sample_id),
        });
    }
    points
}

fn short_sample_label(sample_id: &str) -> String {
    let candidate = Path::new(sample_id)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(sample_id);
    if candidate.chars().count() > 32 {
        let mut truncated = candidate.chars().take(29).collect::<String>();
        truncated.push_str("...");
        truncated
    } else {
        candidate.to_string()
    }
}

fn hash_scalar<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
