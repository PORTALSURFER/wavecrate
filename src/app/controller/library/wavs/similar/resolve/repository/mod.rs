//! Focused source lookup, batched loading, and decode helpers for similarity resolution.

mod decode;
mod loaders;
mod source_lookup;

#[cfg(test)]
mod tests;

pub(crate) use loaders::{
    load_embeddings_for_samples, load_feature_metrics_for_samples, load_query_similarity_inputs,
    load_rms_for_samples,
};
pub(crate) use source_lookup::{open_source_db_for_id, resolve_sample_id_for_visible_row};
