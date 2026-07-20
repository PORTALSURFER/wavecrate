//! Source-database persistence shared by readiness execution and legacy fixtures.

mod artifacts;
mod ids;
mod sample_metadata;
mod samples;
pub(crate) mod telemetry;
mod types;

pub(crate) use artifacts::{
    AnalysisMetadataUpdate, AspectDescriptorUpsert, CachedAspectDescriptorsUpsert,
    CachedEmbeddingUpsert, CachedFeatures, CachedFeaturesUpsert, EmbeddingUpsert,
    cached_aspect_descriptors_by_hash, cached_embedding_by_hash, cached_features_by_hash,
    update_analysis_metadata, upsert_analysis_features, upsert_aspect_descriptors,
    upsert_cached_aspect_descriptors, upsert_cached_embedding, upsert_cached_features,
    upsert_embedding,
};
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) use artifacts::{update_sample_duration, update_sample_long_mark};
pub(crate) use ids::{build_sample_id, parse_sample_id};
pub(crate) use sample_metadata::sample_content_hash;
#[cfg(test)]
pub(crate) use sample_metadata::{sample_bpm, update_sample_bpm, update_sample_bpms};
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) use sample_metadata::{sample_ids_missing_duration, update_sample_bpms_in_tx};
#[cfg(test)]
pub(crate) use samples::upsert_samples;
pub(crate) use samples::upsert_samples_in_tx;
pub(crate) use types::SampleMetadata;
