use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::db::telemetry;
use std::path::Path;

use super::ann::upsert_ann_with_recovery;
use super::planning::DecodedAnalysisWrite;

pub(crate) fn apply_cached_features_and_embedding(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    content_hash: &str,
    features: &db::CachedFeatures,
    embedding: &db::CachedEmbedding,
    aspect_descriptors: Option<&db::CachedAspectDescriptors>,
    embedding_vec: &[f32],
    analysis_version: &str,
) -> Result<(), String> {
    if !persist_cached_analysis_write(
        conn,
        job,
        Some(job.source_root.as_path()),
        content_hash,
        features,
        embedding,
        aspect_descriptors,
        analysis_version,
    )? {
        return Ok(());
    }
    upsert_ann_with_recovery(conn, job, embedding_vec)?;
    Ok(())
}

pub(crate) fn apply_cached_embedding(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    embedding: &db::CachedEmbedding,
) -> Result<(), String> {
    db::upsert_embedding(
        conn,
        db::EmbeddingUpsert {
            sample_id: &job.sample_id,
            model_id: &embedding.model_id,
            dim: embedding.dim,
            dtype: &embedding.dtype,
            l2_normed: embedding.l2_normed,
            vec_blob: &embedding.vec_blob,
            created_at: embedding.created_at,
        },
    )?;
    Ok(())
}

pub(crate) fn update_metadata_for_skip(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    duration_seconds: f32,
    sample_rate: u32,
    analysis_version: &str,
) -> Result<(), String> {
    db::update_analysis_metadata(
        conn,
        db::AnalysisMetadataUpdate {
            sample_id: &job.sample_id,
            content_hash: job.content_hash.as_deref(),
            duration_seconds,
            sr_used: sample_rate,
            analysis_version,
        },
    )
}

/// Persist one decoded analysis result inside a single immediate transaction.
pub(crate) fn persist_decoded_analysis_write(
    conn: &mut rusqlite::Connection,
    source_root: Option<&Path>,
    write: &DecodedAnalysisWrite,
) -> Result<bool, String> {
    let mut persisted =
        persist_decoded_analysis_batch(conn, source_root, std::slice::from_ref(write))?;
    Ok(persisted.pop().unwrap_or(false))
}

/// Persist one decoded batch inside a single immediate transaction.
pub(crate) fn persist_decoded_analysis_batch(
    conn: &mut rusqlite::Connection,
    source_root: Option<&Path>,
    writes: &[DecodedAnalysisWrite],
) -> Result<Vec<bool>, String> {
    if writes.is_empty() {
        return Ok(Vec::new());
    }
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_persist_decoded_batch")
        .map_err(|err| format!("Failed to start decoded analysis transaction: {err}"))?;
    let mut persisted = Vec::with_capacity(writes.len());
    for write in writes {
        persisted.push(persist_decoded_analysis_write_in_tx(&tx, write)?);
    }
    telemetry::commit_transaction(tx, "analysis_persist_decoded_batch")
        .map_err(|err| format!("Failed to commit decoded analysis transaction: {err}"))?;
    maybe_checkpoint_source_db(source_root);
    Ok(persisted)
}

/// Finish one decoded analysis write by updating ANN state after the SQL commit.
pub(crate) fn finish_decoded_analysis_write(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    write: &DecodedAnalysisWrite,
    persisted: bool,
    do_ann_upsert: bool,
) -> Result<(), String> {
    if !persisted || !do_ann_upsert {
        return Ok(());
    }
    upsert_ann_with_recovery(conn, job, &write.ann_embedding)
}

fn persist_decoded_analysis_write_in_tx(
    conn: &rusqlite::Connection,
    write: &DecodedAnalysisWrite,
) -> Result<bool, String> {
    if db::sample_content_hash(conn, &write.sample_id)?.as_deref() != Some(&write.content_hash) {
        return Ok(false);
    }
    if write.needs_embedding_upsert {
        db::upsert_embedding(conn, write.embedding_upsert())?;
    }
    db::update_analysis_metadata(conn, write.metadata_update())?;
    db::upsert_analysis_features(
        conn,
        &write.sample_id,
        &write.feature_blob,
        write.light_dsp_blob.as_deref(),
        write.rms,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
        write.computed_at,
    )?;
    db::upsert_aspect_descriptors(conn, write.aspect_descriptor_upsert())?;
    db::upsert_cached_features(conn, write.cached_features_upsert())?;
    db::upsert_cached_embedding(conn, write.cached_embedding_upsert())?;
    db::upsert_cached_aspect_descriptors(conn, write.cached_aspect_descriptor_upsert())?;
    Ok(true)
}

fn persist_cached_analysis_write(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    source_root: Option<&Path>,
    content_hash: &str,
    features: &db::CachedFeatures,
    embedding: &db::CachedEmbedding,
    aspect_descriptors: Option<&db::CachedAspectDescriptors>,
    analysis_version: &str,
) -> Result<bool, String> {
    let aspect_write = cached_aspect_descriptor_write(features, aspect_descriptors)?;
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_persist_cached")
        .map_err(|err| format!("Failed to start cached analysis transaction: {err}"))?;
    if db::sample_content_hash(&tx, &job.sample_id)?.as_deref() != Some(content_hash) {
        telemetry::commit_transaction(tx, "analysis_persist_cached")
            .map_err(|err| format!("Failed to commit cached analysis skip: {err}"))?;
        return Ok(false);
    }
    db::update_analysis_metadata(
        &tx,
        db::AnalysisMetadataUpdate {
            sample_id: &job.sample_id,
            content_hash: Some(content_hash),
            duration_seconds: features.duration_seconds,
            sr_used: features.sr_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        &tx,
        &job.sample_id,
        &features.vec_blob,
        features.light_dsp_blob.as_deref(),
        features.rms,
        features.feat_version,
        features.computed_at,
    )?;
    db::upsert_embedding(
        &tx,
        db::EmbeddingUpsert {
            sample_id: &job.sample_id,
            model_id: &embedding.model_id,
            dim: embedding.dim,
            dtype: &embedding.dtype,
            l2_normed: embedding.l2_normed,
            vec_blob: &embedding.vec_blob,
            created_at: embedding.created_at,
        },
    )?;
    db::upsert_aspect_descriptors(
        &tx,
        db::AspectDescriptorUpsert {
            sample_id: &job.sample_id,
            model_id: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            dim: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            dtype: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            l2_normed: true,
            valid_mask: aspect_write.valid_mask,
            vec_blob: &aspect_write.vec_blob,
            created_at: aspect_write.created_at,
        },
    )?;
    db::upsert_cached_aspect_descriptors(
        &tx,
        db::CachedAspectDescriptorsUpsert {
            content_hash,
            analysis_version,
            model_id: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            dim: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            dtype: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            l2_normed: true,
            valid_mask: aspect_write.valid_mask,
            vec_blob: &aspect_write.vec_blob,
            created_at: aspect_write.created_at,
        },
    )?;
    telemetry::commit_transaction(tx, "analysis_persist_cached")
        .map_err(|err| format!("Failed to commit cached analysis transaction: {err}"))?;
    maybe_checkpoint_source_db(source_root);
    Ok(true)
}

struct AspectDescriptorWrite {
    vec_blob: Vec<u8>,
    valid_mask: u32,
    created_at: i64,
}

fn cached_aspect_descriptor_write(
    features: &db::CachedFeatures,
    cached: Option<&db::CachedAspectDescriptors>,
) -> Result<AspectDescriptorWrite, String> {
    if let Some(cached) = cached.filter(|cached| cached_aspect_descriptor_is_current(cached)) {
        return Ok(AspectDescriptorWrite {
            vec_blob: cached.vec_blob.clone(),
            valid_mask: cached.valid_mask,
            created_at: cached.created_at,
        });
    }
    let feature_values = wavecrate_analysis::decode_f32_le_blob(&features.vec_blob)?;
    let descriptors =
        wavecrate_analysis::aspects::aspect_descriptors_from_features_v1(&feature_values)?;
    Ok(AspectDescriptorWrite {
        vec_blob: wavecrate_analysis::vector::encode_f32_le_blob(descriptors.packed()),
        valid_mask: descriptors.valid_mask(),
        created_at: features.computed_at,
    })
}

fn cached_aspect_descriptor_is_current(cached: &db::CachedAspectDescriptors) -> bool {
    cached.model_id == wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID
        && cached.dim == wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64
        && cached.dtype == wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32
        && cached.l2_normed
        && cached.vec_blob.len() == wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4
}

fn maybe_checkpoint_source_db(source_root: Option<&Path>) {
    let Some(source_root) = source_root else {
        return;
    };
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
}
