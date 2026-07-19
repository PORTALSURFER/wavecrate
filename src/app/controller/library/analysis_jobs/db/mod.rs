/// Analysis artifact persistence helpers and typed upsert requests.
pub(crate) mod artifacts;
mod cleanup;
mod connection;
mod ids;
mod progress;
#[path = "jobs/sample_metadata.rs"]
mod sample_metadata;
mod samples;
pub(crate) mod telemetry;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::artifacts::{
    AnalysisMetadataUpdate, AspectDescriptorUpsert, CachedAspectDescriptorsUpsert,
    CachedEmbeddingUpsert, CachedFeatures, CachedFeaturesUpsert, EmbeddingUpsert,
    cached_aspect_descriptors_by_hash, cached_embedding_by_hash, cached_features_by_hash,
    update_analysis_metadata, update_sample_duration, update_sample_long_mark,
    upsert_analysis_features, upsert_aspect_descriptors, upsert_cached_aspect_descriptors,
    upsert_cached_embedding, upsert_cached_features, upsert_embedding,
};
pub(crate) use cleanup::purge_orphaned_samples_in_tx;
pub(crate) use connection::{
    AnalysisJobSession, AnalysisReadSession, open_source_db, open_source_db_background_read,
    open_source_db_maintenance, open_source_db_ui_read,
};
pub(crate) use ids::{build_sample_id, parse_sample_id};
pub(crate) use progress::has_pending_or_running_jobs;
#[cfg(test)]
pub(crate) use sample_metadata::{sample_bpm, update_sample_bpm, update_sample_bpms};
pub(crate) use sample_metadata::{
    sample_content_hash, sample_ids_missing_duration, update_sample_bpms_in_tx,
};
#[cfg(test)]
pub(crate) use samples::upsert_samples;
pub(crate) use samples::upsert_samples_in_tx;
pub(crate) use types::SampleMetadata;
