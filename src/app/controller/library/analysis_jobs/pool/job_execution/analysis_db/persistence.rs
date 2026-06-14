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

pub(crate) fn finalize_analysis_job(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    decoded: wavecrate_analysis::AnalysisAudio,
    analysis_version: &str,
    needs_embedding_upsert: bool,
    do_ann_upsert: bool,
) -> Result<(), String> {
    let write = super::build_decoded_analysis_write(
        job,
        decoded,
        analysis_version,
        needs_embedding_upsert,
    )?;
    let persisted = persist_decoded_analysis_write(conn, Some(job.source_root.as_path()), &write)?;
    finish_decoded_analysis_write(conn, job, &write, persisted, do_ann_upsert)
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
    db::upsert_cached_features(conn, write.cached_features_upsert())?;
    db::upsert_cached_embedding(conn, write.cached_embedding_upsert())?;
    Ok(true)
}

fn persist_cached_analysis_write(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    source_root: Option<&Path>,
    content_hash: &str,
    features: &db::CachedFeatures,
    embedding: &db::CachedEmbedding,
    analysis_version: &str,
) -> Result<bool, String> {
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
    telemetry::commit_transaction(tx, "analysis_persist_cached")
        .map_err(|err| format!("Failed to commit cached analysis transaction: {err}"))?;
    maybe_checkpoint_source_db(source_root);
    Ok(true)
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
