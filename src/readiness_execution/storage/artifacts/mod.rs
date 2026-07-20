mod content_cache;
mod current_artifacts;
mod sample_metadata;

pub(crate) use content_cache::{
    CachedAspectDescriptorsUpsert, CachedEmbeddingUpsert, CachedFeatures, CachedFeaturesUpsert,
    cached_aspect_descriptors_by_hash, cached_embedding_by_hash, cached_features_by_hash,
    upsert_cached_aspect_descriptors, upsert_cached_embedding, upsert_cached_features,
};
pub(crate) use current_artifacts::{
    AspectDescriptorUpsert, EmbeddingUpsert, upsert_analysis_features, upsert_aspect_descriptors,
    upsert_embedding,
};
pub(crate) use sample_metadata::{AnalysisMetadataUpdate, update_analysis_metadata};
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) use sample_metadata::{update_sample_duration, update_sample_long_mark};
