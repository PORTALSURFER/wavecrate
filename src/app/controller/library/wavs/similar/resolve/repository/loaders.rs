//! Batched SQLite loaders for similarity embeddings and feature metrics.

mod batch;
mod embedding_loader;
mod feature_metrics_loader;
mod query_inputs;
mod rms_loader;

#[cfg(test)]
pub(crate) use batch::SQLITE_IN_BATCH_SIZE;
pub(crate) use embedding_loader::load_embeddings_for_samples;
pub(crate) use feature_metrics_loader::{
    SimilarityFeatureMetrics, load_feature_metrics_for_samples,
};
pub(crate) use query_inputs::load_query_similarity_inputs;
#[cfg(test)]
pub(crate) use rms_loader::load_rms_for_sample;
pub(crate) use rms_loader::load_rms_for_samples;
