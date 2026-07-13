//! Similarity-map build and clustering job orchestration.

use crate::app::controller::library::similarity_prep::DEFAULT_CLUSTER_MIN_SIZE;

use super::connections::{StarmapWriteSession, open_source_db_for_id};
use super::*;

impl AppController {
    /// Enqueue a Starmap layout build for the selected source.
    pub fn build_umap_layout(&mut self, model_id: &str, umap_version: &str) {
        if self.runtime.jobs.umap_build_in_progress() {
            self.set_status_message(StatusMessage::StarmapLayoutBuildAlreadyRunning);
            return;
        }
        let Some(source_id) = self.current_source().map(|source| source.id) else {
            self.set_status_message(StatusMessage::SelectSourceFirst {
                tone: StatusTone::Warning,
            });
            return;
        };
        self.runtime
            .jobs
            .begin_umap_build(super::super::jobs::UmapBuildJob {
                model_id: model_id.to_string(),
                umap_version: umap_version.to_string(),
                source_id,
            });
        self.set_status_message(StatusMessage::BuildingStarmapLayout);
    }

    /// Enqueue cluster generation for the current starmap layout.
    pub fn build_umap_clusters(&mut self, model_id: &str, umap_version: &str) {
        if self.runtime.jobs.umap_cluster_build_in_progress() {
            self.set_status_message(StatusMessage::ClusterBuildAlreadyRunning);
            return;
        }
        let source_id = self.current_source().map(|source| source.id);
        self.runtime
            .jobs
            .begin_umap_cluster_build(super::super::jobs::UmapClusterBuildJob {
                model_id: model_id.to_string(),
                umap_version: umap_version.to_string(),
                source_id,
            });
        self.set_status_message(StatusMessage::BuildingClusters);
    }
}

pub(crate) fn run_umap_build(
    model_id: &str,
    umap_version: &str,
    source_id: &SourceId,
) -> Result<super::super::jobs::StarmapWriteOutcome<()>, String> {
    let StarmapWriteSession::Ready(mut conn) = open_source_db_for_id(source_id)? else {
        return Ok(super::super::jobs::StarmapWriteOutcome::DeferredForFileOp);
    };
    wavecrate_analysis::build_map_layout(&mut conn, model_id, umap_version, 0, 0.95)?;
    Ok(super::super::jobs::StarmapWriteOutcome::Completed(()))
}

pub(crate) fn run_umap_cluster_build(
    model_id: &str,
    umap_version: &str,
    source_id: Option<&SourceId>,
) -> Result<
    super::super::jobs::StarmapWriteOutcome<wavecrate_analysis::hdbscan::HdbscanStats>,
    String,
> {
    let Some(source_id) = source_id else {
        return Err("Missing source for cluster build".to_string());
    };
    let StarmapWriteSession::Ready(mut conn) = open_source_db_for_id(source_id)? else {
        return Ok(super::super::jobs::StarmapWriteOutcome::DeferredForFileOp);
    };
    let sample_id_prefix = Some(format!("{}::%", source_id.as_str()));
    wavecrate_analysis::hdbscan::build_hdbscan_clusters_for_sample_id_prefix(
        &mut conn,
        model_id,
        wavecrate_analysis::hdbscan::HdbscanMethod::Umap,
        Some(umap_version),
        sample_id_prefix.as_deref(),
        wavecrate_analysis::hdbscan::HdbscanConfig {
            min_cluster_size: DEFAULT_CLUSTER_MIN_SIZE,
            min_samples: None,
            allow_single_cluster: false,
        },
    )
    .map(super::super::jobs::StarmapWriteOutcome::Completed)
}
