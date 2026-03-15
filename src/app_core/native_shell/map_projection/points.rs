use super::super::waveform_projection::normalized_to_milli;
use super::*;
use std::collections::HashSet;

/// Refresh retained normalized map-point cache only when projection key changes.
pub(super) fn refresh_projected_map_points_cache(
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
) -> (Arc<[ProjectedMapPointCacheEntry]>, usize) {
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
            sample_id: Arc::clone(&point.sample_id),
            x_milli: normalized_to_milli(x),
            y_milli: normalized_to_milli(y),
            cluster_id: point.cluster_id,
        });
    }
    (Arc::from(projected_points), cluster_ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_map_points_normalize_into_milli_space_and_count_clusters() {
        let bounds = MapBounds {
            min_x: -1.0,
            max_x: 1.0,
            min_y: -2.0,
            max_y: 2.0,
        };
        let points = vec![
            MapPoint {
                sample_id: Arc::<str>::from("a"),
                x: -1.0,
                y: -2.0,
                cluster_id: Some(1),
            },
            MapPoint {
                sample_id: Arc::<str>::from("b"),
                x: 1.0,
                y: 2.0,
                cluster_id: Some(2),
            },
            MapPoint {
                sample_id: Arc::<str>::from("c"),
                x: 0.0,
                y: 0.0,
                cluster_id: Some(2),
            },
        ];

        let (projected, cluster_count) = build_projected_map_points_cache(bounds, &points);

        assert_eq!(cluster_count, 2);
        assert_eq!(projected[0].x_milli, 0);
        assert_eq!(projected[0].y_milli, 0);
        assert_eq!(projected[1].x_milli, 1000);
        assert_eq!(projected[1].y_milli, 1000);
        assert_eq!(projected[2].x_milli, 500);
        assert_eq!(projected[2].y_milli, 500);
    }
}
