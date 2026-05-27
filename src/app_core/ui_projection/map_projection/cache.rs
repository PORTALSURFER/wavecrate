use super::*;
use crate::sample_sources::SourceId;
use std::hash::{Hash, Hasher};

/// Resolve map bounds from cache or the backing query layer.
pub(super) fn resolve_map_bounds(
    controller: &mut AppController,
    source_id: Option<&SourceId>,
    source_id_key: &Option<String>,
    umap_version: &str,
) -> Result<Option<MapBounds>, String> {
    let has_matching_bounds_cache = controller.ui.map.cached_bounds_source_id == *source_id_key
        && controller.ui.map.cached_bounds_umap_version.as_deref() == Some(umap_version);
    if has_matching_bounds_cache {
        return Ok(controller.ui.map.bounds);
    }

    let bounds = controller.umap_bounds(SIMILARITY_MODEL_ID, umap_version, source_id)?;
    let mapped_bounds = bounds.map(|value| MapBounds {
        min_x: value.min_x,
        max_x: value.max_x,
        min_y: value.min_y,
        max_y: value.max_y,
    });
    controller.ui.map.cached_bounds_source_id = source_id_key.clone();
    controller.ui.map.cached_bounds_umap_version = Some(umap_version.to_string());
    controller.ui.map.bounds = mapped_bounds;
    controller.mark_map_dataset_projection_revision_dirty();
    Ok(mapped_bounds)
}

/// Refresh cached map query points when the current bounds/source/version change.
pub(super) fn refresh_map_points_query_cache(
    controller: &mut AppController,
    source_id: Option<&SourceId>,
    source_id_key: &Option<String>,
    umap_version: &str,
    bounds: MapBounds,
) -> Result<MapQueryBounds, String> {
    let query_bounds = MapQueryBounds {
        min_x: bounds.min_x,
        max_x: bounds.max_x,
        min_y: bounds.min_y,
        max_y: bounds.max_y,
    };
    let has_matching_points_cache = controller.ui.map.cached_points_source_id == *source_id_key
        && controller.ui.map.cached_points_umap_version.as_deref() == Some(umap_version)
        && controller.ui.map.last_query == Some(query_bounds);
    if has_matching_points_cache {
        return Ok(query_bounds);
    }

    let points = controller.umap_points_in_bounds(UmapPointQuery {
        model_id: SIMILARITY_MODEL_ID,
        umap_version,
        cluster_method: "umap",
        cluster_umap_version: umap_version,
        source_id,
        bounds: query_bounds,
        limit: MAX_RENDERED_MAP_POINTS,
    })?;
    controller.ui.map.cached_points = points
        .iter()
        .map(|point| MapPoint {
            sample_id: Arc::<str>::from(point.sample_id.as_str()),
            x: point.x,
            y: point.y,
            cluster_id: point.cluster_id,
        })
        .collect::<Vec<_>>();
    controller.ui.map.cached_points_source_id = source_id_key.clone();
    controller.ui.map.cached_points_umap_version = Some(umap_version.to_string());
    controller.ui.map.last_query = Some(query_bounds);
    controller.ui.map.cached_points_revision =
        controller.ui.map.cached_points_revision.saturating_add(1);
    controller.mark_map_dataset_projection_revision_dirty();
    controller.mark_map_query_projection_revision_dirty();
    Ok(query_bounds)
}

/// Build a retained map-projection cache key from active map source/query state.
pub(super) fn map_projection_cache_key(
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

fn hash_scalar<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
