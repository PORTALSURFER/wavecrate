//! Runtime-maintenance and map-build background-job handlers.

use super::*;
use crate::app::controller::jobs::{
    SourceDbMaintenanceResult, UmapBuildResult, UmapClusterBuildResult,
};

impl AppController {
    /// Apply one completed similarity-map layout build result.
    pub(super) fn handle_umap_built_message(&mut self, message: UmapBuildResult) {
        self.runtime.jobs.clear_umap_build();
        match message.result {
            Ok(()) => {
                self.ui.map.bounds = None;
                self.ui.map.cached_bounds_source_id = None;
                self.ui.map.cached_bounds_umap_version = None;
                self.ui.map.last_query = None;
                self.ui.map.cached_points.clear();
                self.ui.map.cached_points_source_id = None;
                self.ui.map.cached_points_umap_version = None;
                self.mark_map_dataset_projection_revision_dirty();
                self.mark_map_query_projection_revision_dirty();
                self.set_status(
                    format!("Similarity map layout {} built", message.umap_version),
                    StatusTone::Info,
                );
            }
            Err(err) => {
                self.set_status(
                    format!("Similarity map layout build failed: {err}"),
                    StatusTone::Error,
                );
            }
        }
    }

    /// Apply one completed similarity-map cluster-build result.
    pub(super) fn handle_umap_clusters_built_message(&mut self, message: UmapClusterBuildResult) {
        self.runtime.jobs.clear_umap_cluster_build();
        match message.result {
            Ok(stats) => {
                self.ui.map.last_query = None;
                self.ui.map.cached_points.clear();
                self.ui.map.cached_points_source_id = None;
                self.ui.map.cached_points_umap_version = None;
                self.ui.map.cached_cluster_centroids_key = None;
                self.ui.map.cached_cluster_centroids = None;
                self.ui.map.auto_cluster_build_requested_key = None;
                self.mark_map_dataset_projection_revision_dirty();
                self.mark_map_query_projection_revision_dirty();
                let scope = message
                    .source_id
                    .as_ref()
                    .map(|id| id.as_str())
                    .unwrap_or("all sources");
                self.set_status(
                    format!(
                        "Clusters built for {scope} ({} clusters, {:.1}% noise)",
                        stats.cluster_count,
                        stats.noise_ratio * 100.0
                    ),
                    StatusTone::Info,
                );
            }
            Err(err) => {
                self.set_status(format!("Cluster build failed: {err}"), StatusTone::Error);
            }
        }
    }

    /// Apply one deferred source-DB maintenance batch result.
    pub(super) fn handle_source_db_maintenance_finished_message(
        &mut self,
        message: SourceDbMaintenanceResult,
    ) {
        self.runtime.jobs.clear_source_db_maintenance();
        let mut failed = 0usize;
        for outcome in message.outcomes {
            if let Some(err) = outcome.error {
                failed = failed.saturating_add(1);
                tracing::warn!(
                    "Deferred source DB maintenance failed for {} ({}): {}",
                    outcome.source_id,
                    outcome.source_root.display(),
                    err
                );
            }
        }
        if failed > 0 {
            let suffix = if failed == 1 { "" } else { "s" };
            self.set_status(
                format!("Deferred source DB maintenance failed for {failed} source{suffix}"),
                StatusTone::Warning,
            );
        }
    }
}
