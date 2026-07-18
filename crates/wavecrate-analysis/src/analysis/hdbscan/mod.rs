//! HDBSCAN clustering helpers for embeddings and 2D layouts.

mod engine;
mod mapping;

use self::mapping::{assign_all_points_to_clusters, remap_labels_deterministic};

/// Parameters controlling the HDBSCAN clustering pass.
#[derive(Debug, Clone, Copy)]
pub struct HdbscanConfig {
    /// Minimum number of points required to form a cluster.
    pub min_cluster_size: usize,
    /// Optional minimum number of samples for a core point.
    pub min_samples: Option<usize>,
    /// Whether to allow a single cluster containing all points.
    pub allow_single_cluster: bool,
}

pub(crate) fn compute_layout_clusters(
    sample_ids: &[String],
    layout: &[[f32; 2]],
    config: HdbscanConfig,
) -> Result<Vec<i32>, String> {
    if config.min_cluster_size == 0 || config.min_samples == Some(0) {
        return Err("HDBSCAN cluster sizes must be greater than zero".to_string());
    }
    if sample_ids.len() != layout.len() {
        return Err("Cluster layout length mismatch".to_string());
    }
    if layout.is_empty() {
        return Ok(Vec::new());
    }
    let data = layout
        .iter()
        .map(|point| vec![point[0], point[1]])
        .collect::<Vec<_>>();
    let mut labels = engine::run_hdbscan(&data, config)?;
    assign_all_points_to_clusters(&data, &mut labels);
    remap_labels_deterministic(sample_ids, &mut labels)?;
    Ok(labels)
}
