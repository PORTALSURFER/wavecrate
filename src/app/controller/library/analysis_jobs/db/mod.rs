mod ann_index;
/// Analysis artifact persistence helpers and typed upsert requests.
pub(crate) mod artifacts;
mod cleanup;
mod connection;
mod constants;
mod enqueue;
mod ids;
mod jobs;
mod progress;
mod progress_snapshot;
pub(crate) mod telemetry;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::artifacts::{
    AnalysisMetadataUpdate, CachedEmbedding, CachedEmbeddingUpsert, CachedFeatures,
    CachedFeaturesUpsert, EmbeddingUpsert, cached_embedding_by_hash, cached_features_by_hash,
    invalidate_analysis_artifacts_in_tx, update_analysis_metadata, update_sample_duration,
    update_sample_long_mark, upsert_analysis_features, upsert_cached_embedding,
    upsert_cached_features, upsert_embedding,
};
pub(crate) use ann_index::{
    clear_ann_index_dirty, enqueue_rebuild_ann_index_job, mark_ann_index_dirty,
};
#[cfg(test)]
pub(crate) use cleanup::purge_orphaned_samples;
pub(crate) use cleanup::purge_orphaned_samples_in_tx;
#[cfg(test)]
pub(crate) use cleanup::fail_stale_running_jobs;
pub(crate) use cleanup::{
    fail_stale_running_jobs_with_sources, prune_jobs_for_missing_sources, reset_running_to_pending,
};
pub(crate) use connection::{open_source_db, open_source_db_maintenance, open_source_db_ui_read};
#[cfg(test)]
pub(crate) use constants::DEFAULT_JOB_TYPE;
pub(crate) use constants::{
    ANALYZE_SAMPLE_JOB_TYPE, EMBEDDING_BACKFILL_JOB_TYPE, REBUILD_INDEX_JOB_TYPE,
};
#[cfg(test)]
pub(crate) use enqueue::upsert_samples;
pub(crate) use enqueue::{enqueue_jobs, enqueue_jobs_in_tx, upsert_samples_in_tx};
pub(crate) use ids::{build_sample_id, parse_sample_id};
#[cfg(test)]
pub(crate) use jobs::claim_next_job;
#[cfg(test)]
pub(crate) use jobs::sample_bpm;
#[cfg(test)]
pub(crate) use jobs::update_sample_bpm;
#[cfg(test)]
pub(crate) use jobs::update_sample_bpms;
pub(crate) use jobs::{
    SampleAnalysisState, claim_next_jobs, mark_done, mark_failed_with_reason, mark_pending,
    sample_analysis_states, sample_content_hash, sample_ids_missing_duration, touch_running_at,
    update_sample_bpms_in_tx,
};
pub(crate) use progress::{
    current_embedding_backfill_progress, current_progress, current_running_jobs,
};
pub(crate) use types::{ClaimedJob, SampleMetadata};
