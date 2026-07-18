use hdbscan::{Hdbscan, HdbscanHyperParams};

use super::HdbscanConfig;

pub fn run_hdbscan(data: &[Vec<f32>], config: HdbscanConfig) -> Result<Vec<i32>, String> {
    let min_required = config
        .min_samples
        .unwrap_or(1)
        .max(config.min_cluster_size)
        .max(2);
    if data.len() < min_required {
        // HDBSCAN panics on tiny datasets; treat them as a single cluster.
        return Ok(vec![0; data.len()]);
    }
    let hyper = build_hyperparams(config);
    let clusterer = Hdbscan::new(data, hyper);
    clusterer
        .cluster()
        .map_err(|err| format!("HDBSCAN clustering failed: {err}"))
}

fn build_hyperparams(config: HdbscanConfig) -> HdbscanHyperParams {
    let mut builder = HdbscanHyperParams::builder().min_cluster_size(config.min_cluster_size);
    if let Some(min_samples) = config.min_samples {
        builder = builder.min_samples(min_samples);
    }
    if config.allow_single_cluster {
        builder = builder.allow_single_cluster(true);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_single_cluster_for_tiny_datasets() {
        let data = vec![vec![0.0, 1.0], vec![2.0, 3.0]];
        let config = HdbscanConfig {
            min_cluster_size: 5,
            min_samples: None,
            allow_single_cluster: true,
        };
        let labels = run_hdbscan(&data, config).unwrap();
        assert_eq!(labels, vec![0, 0]);
    }
}
