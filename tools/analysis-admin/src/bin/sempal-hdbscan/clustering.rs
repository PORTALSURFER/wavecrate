//! HDBSCAN clustering and label-summary helpers for the support binary.

use super::args::Options;
use hdbscan::{Hdbscan, HdbscanHyperParams};
use std::collections::HashMap;

/// Summary statistics for one HDBSCAN run.
pub(super) struct LabelStats {
    pub(super) cluster_count: usize,
    pub(super) noise_count: usize,
    pub(super) noise_ratio: f32,
    pub(super) min_cluster_size: usize,
    pub(super) max_cluster_size: usize,
}

/// Run HDBSCAN with the configured options and summarize the resulting labels.
pub(super) fn cluster_embeddings(
    data: &[Vec<f32>],
    options: &Options,
) -> Result<(Vec<i32>, LabelStats), String> {
    let mut builder = HdbscanHyperParams::builder().min_cluster_size(options.min_cluster_size);
    if let Some(min_samples) = options.min_samples {
        builder = builder.min_samples(min_samples);
    }
    if options.allow_single_cluster {
        builder = builder.allow_single_cluster(true);
    }
    let labels = Hdbscan::new(data, builder.build())
        .cluster()
        .map_err(|err| format!("HDBSCAN clustering failed: {err}"))?;
    let stats = summarize_labels(&labels);
    options.noise_policy.handle_ratio(
        stats.noise_ratio,
        options.min_noise_ratio,
        options.max_noise_ratio,
    )?;
    Ok((labels, stats))
}

fn summarize_labels(labels: &[i32]) -> LabelStats {
    let mut cluster_counts: HashMap<i32, usize> = HashMap::new();
    let mut noise = 0usize;
    for label in labels {
        if *label < 0 {
            noise += 1;
        } else {
            *cluster_counts.entry(*label).or_insert(0) += 1;
        }
    }
    let total = labels.len().max(1) as f32;
    let (min_cluster_size, max_cluster_size) = if cluster_counts.is_empty() {
        (0, 0)
    } else {
        let mut min_size = usize::MAX;
        let mut max_size = 0usize;
        for size in cluster_counts.values() {
            min_size = min_size.min(*size);
            max_size = max_size.max(*size);
        }
        (min_size, max_size)
    };
    LabelStats {
        cluster_count: cluster_counts.len(),
        noise_count: noise,
        noise_ratio: noise as f32 / total,
        min_cluster_size,
        max_cluster_size,
    }
}
