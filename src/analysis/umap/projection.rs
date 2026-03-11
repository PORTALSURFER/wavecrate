//! Similarity-map projection math backed by PCA+t-SNE.

use linfa::dataset::DatasetBase;
use linfa::traits::{Fit, Transformer};
use linfa_reduction::Pca;
use linfa_tsne::TSneParams;
use ndarray::Array2;
use rand_08::SeedableRng;
use rand_08::rngs::SmallRng;

use super::LayoutPoint;

const DEFAULT_PERPLEXITY: f64 = 35.0;
const DEFAULT_APPROX_THRESHOLD: f64 = 0.5;
const DEFAULT_MAX_ITER: usize = 1500;
const DEFAULT_N_COMPONENTS: usize = 2;
const DEFAULT_PCA_COMPONENTS: usize = 50;

/// Compute one 2D similarity-map projection from flattened embedding vectors.
pub(super) fn compute_tsne(
    vectors: Vec<f64>,
    dim: usize,
    seed: u64,
) -> Result<Vec<LayoutPoint>, String> {
    let n_samples = sample_count(&vectors, dim)?;
    let matrix = build_embedding_matrix(vectors, n_samples, dim)?;
    let reduced = reduce_with_pca_if_needed(matrix, dim, n_samples)?;
    project_embedding(reduced, n_samples, seed)
}

fn sample_count(vectors: &[f64], dim: usize) -> Result<usize, String> {
    if dim == 0 || vectors.len() % dim != 0 {
        return Err("Similarity map embedding matrix shape mismatch".to_string());
    }
    let n_samples = vectors.len() / dim;
    if n_samples < 2 {
        return Err("Need at least 2 embeddings to build a similarity map layout".to_string());
    }
    Ok(n_samples)
}

fn build_embedding_matrix(
    vectors: Vec<f64>,
    n_samples: usize,
    dim: usize,
) -> Result<Array2<f64>, String> {
    Array2::from_shape_vec((n_samples, dim), vectors)
        .map_err(|err| format!("Build embedding matrix failed: {err}"))
}

fn reduce_with_pca_if_needed(
    matrix: Array2<f64>,
    dim: usize,
    n_samples: usize,
) -> Result<Array2<f64>, String> {
    if dim <= DEFAULT_PCA_COMPONENTS {
        return Ok(matrix);
    }
    let pca_components = resolve_pca_components(dim, n_samples)?;
    let dataset = DatasetBase::from(matrix);
    let pca = Pca::params(pca_components)
        .fit(&dataset)
        .map_err(|err| format!("PCA fit failed: {err}"))?;
    let reduced = pca.transform(dataset);
    Ok(reduced.records)
}

fn resolve_pca_components(dim: usize, n_samples: usize) -> Result<usize, String> {
    let pca_components = DEFAULT_PCA_COMPONENTS
        .min(dim)
        .min(n_samples.saturating_sub(1).max(1));
    if pca_components < 2 {
        return Err("Need at least 2 samples for PCA reduction".to_string());
    }
    Ok(pca_components)
}

fn project_embedding(
    matrix: Array2<f64>,
    n_samples: usize,
    seed: u64,
) -> Result<Vec<LayoutPoint>, String> {
    let rng = SmallRng::seed_from_u64(seed);
    let embedding = TSneParams::embedding_size_with_rng(DEFAULT_N_COMPONENTS, rng)
        .perplexity(resolve_perplexity(n_samples))
        .approx_threshold(DEFAULT_APPROX_THRESHOLD)
        .max_iter(DEFAULT_MAX_ITER)
        .transform(matrix)
        .map_err(|err| format!("Similarity map layout projection failed: {err}"))?;

    Ok(embedding
        .rows()
        .into_iter()
        .map(|row| [row[0] as f32, row[1] as f32])
        .collect())
}

fn resolve_perplexity(n_samples: usize) -> f64 {
    let max_perplexity = ((n_samples as f64) - 1.0).max(1.0) / 3.0;
    DEFAULT_PERPLEXITY.min(max_perplexity).max(1.0)
}
