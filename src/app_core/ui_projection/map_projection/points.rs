use super::super::waveform_projection::normalized_to_milli;
use crate::app_core::controller::AppController;
use crate::app_core::map_projection_contracts::{
    ProjectedMapPointCacheEntry, ProjectedMapPointsCacheKey,
};
use crate::app_core::state::{MapBounds, MapPoint};
use std::collections::HashSet;
use std::sync::Arc;

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
    let projection = AspectPreservingMapProjection::new(bounds);
    let mut cluster_ids = HashSet::new();
    let mut projected_points = Vec::with_capacity(points.len());
    for point in points {
        if let Some(cluster_id) = point.cluster_id {
            cluster_ids.insert(cluster_id);
        }
        let (x, y) = projection.project(point.x, point.y);
        projected_points.push(ProjectedMapPointCacheEntry {
            id: Arc::clone(&point.sample_id),
            x_milli: normalized_to_milli(x),
            y_milli: normalized_to_milli(y),
            cluster_id: point.cluster_id,
        });
    }
    (Arc::from(projected_points), cluster_ids.len())
}

#[derive(Clone, Copy)]
struct AspectPreservingMapProjection {
    raw_center_x: f32,
    raw_center_y: f32,
    raw_units_per_normalized_unit: f32,
}

impl AspectPreservingMapProjection {
    fn new(bounds: MapBounds) -> Self {
        let raw_center_x = (bounds.min_x + bounds.max_x) * 0.5;
        let raw_center_y = (bounds.min_y + bounds.max_y) * 0.5;
        let span_x = (bounds.max_x - bounds.min_x).abs();
        let span_y = (bounds.max_y - bounds.min_y).abs();
        let raw_units_per_normalized_unit = span_x.max(span_y);
        Self {
            raw_center_x,
            raw_center_y,
            raw_units_per_normalized_unit,
        }
    }

    fn project(self, x: f32, y: f32) -> (f32, f32) {
        if !x.is_finite() || !y.is_finite() || self.raw_units_per_normalized_unit <= f32::EPSILON {
            return (0.5, 0.5);
        }
        (
            (0.5 + (x - self.raw_center_x) / self.raw_units_per_normalized_unit).clamp(0.0, 1.0),
            (0.5 + (y - self.raw_center_y) / self.raw_units_per_normalized_unit).clamp(0.0, 1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_map_points_preserve_raw_shape_in_milli_space_and_count_clusters() {
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
        assert_eq!(projected[0].x_milli, 250);
        assert_eq!(projected[0].y_milli, 0);
        assert_eq!(projected[1].x_milli, 750);
        assert_eq!(projected[1].y_milli, 1000);
        assert_eq!(projected[2].x_milli, 500);
        assert_eq!(projected[2].y_milli, 500);
    }

    #[test]
    fn projected_map_points_do_not_stretch_tiny_sets_into_rectangle() {
        let bounds = MapBounds {
            min_x: 0.0,
            max_x: 0.10,
            min_y: 0.0,
            max_y: 8.0,
        };
        let points = vec![
            MapPoint {
                sample_id: Arc::<str>::from("a"),
                x: 0.0,
                y: 0.0,
                cluster_id: None,
            },
            MapPoint {
                sample_id: Arc::<str>::from("b"),
                x: 0.10,
                y: 8.0,
                cluster_id: None,
            },
        ];

        let (projected, cluster_count) = build_projected_map_points_cache(bounds, &points);

        assert_eq!(cluster_count, 0);
        assert_eq!(projected[0].x_milli, 494);
        assert_eq!(projected[0].y_milli, 0);
        assert_eq!(projected[1].x_milli, 506);
        assert_eq!(projected[1].y_milli, 1000);
    }
}
