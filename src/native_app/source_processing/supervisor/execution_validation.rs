use super::{
    AtomicBool, ReadinessStage, ReadinessStore, ReadinessTarget, SampleSource, SourceDatabase,
    params, sync_paths_with_progress,
};

pub(super) fn readiness_stage_is_unsupported(
    connection: &mut rusqlite::Connection,
    target: &ReadinessTarget,
    stage: &str,
) -> Result<bool, String> {
    let stage = match stage {
        "analysis_features" => ReadinessStage::AnalysisFeatures,
        "embedding_aspects" => ReadinessStage::EmbeddingAspects,
        _ => return Ok(false),
    };
    ReadinessStore::new(connection)
        .stage_is_unsupported(target, stage)
        .map_err(|error| error.to_string())
}

pub(super) fn reconcile_stale_analysis_input(
    source: &SampleSource,
    relative_path: &std::path::Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let db =
        SourceDatabase::open_for_background_job_with_database_root(&source.root, database_root)
            .map_err(|error| error.to_string())?;
    let stats = sync_paths_with_progress(
        &db,
        &[relative_path.to_path_buf()],
        Some(cancel),
        &mut |_, _| {},
    )
    .map_err(|error| error.to_string())?;
    tracing::info!(
        target: "wavecrate::source_processing",
        source_id = source.id.as_str(),
        path = %relative_path.display(),
        revision = stats.committed_delta.revision,
        changed = stats.committed_delta.changed.len(),
        "Reconciled stale analysis input against the source manifest"
    );
    Ok(())
}

pub(super) fn analysis_features_are_current(
    connection: &rusqlite::Connection,
    target: &ReadinessTarget,
) -> Result<bool, String> {
    let Some(sample_id) = readiness_sample_id(target) else {
        return Ok(false);
    };
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM analysis_cache_features
                WHERE content_hash = ?1 AND analysis_version = ?2
            ) AND EXISTS(
                SELECT 1
                FROM samples AS sample
                JOIN features AS feature ON feature.sample_id = sample.sample_id
                WHERE sample.sample_id = ?3
                  AND sample.content_hash = ?1
                  AND sample.analysis_version = ?2
                  AND feature.feat_version = ?4
            )",
            params![
                target.content_generation,
                target.required_version,
                sample_id,
                wavecrate_analysis::vector::FEATURE_VERSION_V1,
            ],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

pub(super) fn embedding_aspects_are_current(
    connection: &rusqlite::Connection,
    target: &ReadinessTarget,
) -> Result<bool, String> {
    let Some(sample_id) = readiness_sample_id(target) else {
        return Ok(false);
    };
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM samples AS sample
                JOIN embeddings AS embedding
                  ON embedding.sample_id = sample.sample_id
                JOIN analysis_cache_embeddings AS cached_embedding
                  ON cached_embedding.content_hash = sample.content_hash
                 AND cached_embedding.analysis_version = ?2
                 AND cached_embedding.model_id = ?3
                JOIN similarity_aspect_descriptors AS aspects
                  ON aspects.sample_id = sample.sample_id
                JOIN analysis_cache_aspect_descriptors AS cached_aspects
                  ON cached_aspects.content_hash = sample.content_hash
                 AND cached_aspects.analysis_version = ?2
                 AND cached_aspects.model_id = ?4
                WHERE sample.sample_id = ?5
                  AND sample.content_hash = ?1
                  AND embedding.model_id = cached_embedding.model_id
                  AND embedding.dim = cached_embedding.dim
                  AND embedding.dtype = cached_embedding.dtype
                  AND embedding.l2_normed = cached_embedding.l2_normed
                  AND embedding.vec = cached_embedding.vec
                  AND embedding.dim = ?6
                  AND embedding.dtype = ?7
                  AND embedding.l2_normed = 1
                  AND aspects.model_id = cached_aspects.model_id
                  AND aspects.dim = cached_aspects.dim
                  AND aspects.dtype = cached_aspects.dtype
                  AND aspects.l2_normed = cached_aspects.l2_normed
                  AND aspects.valid_mask = cached_aspects.valid_mask
                  AND aspects.vec = cached_aspects.vec
                  AND aspects.dim = ?8
                  AND aspects.dtype = ?9
                  AND aspects.l2_normed = 1
            )",
            params![
                target.content_generation,
                wavecrate_analysis::analysis_version(),
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                sample_id,
                wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
                wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            ],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

pub(super) fn readiness_sample_id(target: &ReadinessTarget) -> Option<String> {
    target
        .relative_path
        .as_deref()
        .map(|relative_path| format!("{}::{}", target.source_id, relative_path.replace('\\', "/")))
}
