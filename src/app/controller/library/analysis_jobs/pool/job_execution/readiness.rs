//! Stage-specific execution reused by the native readiness supervisor.

use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::app::controller::library::analysis_jobs::db;
use rusqlite::OptionalExtension;

use super::{
    analysis::AnalysisContext,
    analysis_decode::{self, DecodeOutcome},
    backfill,
    support::now_epoch_seconds,
};

const FEATURE_RMS_INDEX: usize = 2;

pub(crate) fn run_feature_stage(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    checkpoint(cancel, "feature analysis cancelled")?;
    let sample_id = db::build_sample_id(source_id, relative_path);
    if !ensure_current_sample_row(conn, &sample_id, relative_path, content_hash, source_root)? {
        return Ok(false);
    }
    if db::sample_content_hash(conn, &sample_id)?.as_deref() != Some(content_hash) {
        return Ok(false);
    }
    if let Some(cached) = db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )? {
        return materialize_cached_features(
            conn,
            source_root,
            &sample_id,
            content_hash,
            analysis_version,
            &cached,
        );
    }
    let job = db::ClaimedJob {
        id: -1,
        sample_id: sample_id.clone(),
        content_hash: Some(content_hash.to_string()),
        job_type: db::ANALYZE_SAMPLE_JOB_TYPE.to_string(),
        source_root: source_root.to_path_buf(),
    };
    let context = AnalysisContext {
        use_cache: true,
        max_analysis_duration_seconds: f32::INFINITY,
        analysis_sample_rate: wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
        analysis_version,
        cancel: Some(cancel),
    };
    let decoded = match analysis_decode::decode_for_analysis(&job, &context)? {
        DecodeOutcome::Decoded(decoded) => decoded,
        DecodeOutcome::Skipped { .. } => {
            return Err(String::from(
                "feature analysis unexpectedly skipped an unbounded readiness target",
            ));
        }
    };
    checkpoint(cancel, "feature analysis cancelled before computation")?;
    let features = wavecrate_analysis::compute_feature_vector_v1_for_decoded_audio(&decoded)?;
    checkpoint(cancel, "feature analysis cancelled before publication")?;
    let feature_blob = wavecrate_analysis::vector::encode_f32_le_blob(&features);
    let light_dsp_blob = wavecrate_analysis::light_dsp_from_features_v1(&features)
        .map(|values| wavecrate_analysis::vector::encode_f32_le_blob(&values));
    let rms = features.get(FEATURE_RMS_INDEX).copied();
    let computed_at = now_epoch_seconds();
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_feature_publish")
        .map_err(|error| format!("Failed to start readiness feature transaction: {error}"))?;
    if db::sample_content_hash(&tx, &sample_id)?.as_deref() != Some(content_hash) {
        db::telemetry::commit_transaction(tx, "readiness_feature_publish_stale")
            .map_err(|error| format!("Failed to commit stale feature skip: {error}"))?;
        return Ok(false);
    }
    db::update_analysis_metadata(
        &tx,
        db::AnalysisMetadataUpdate {
            sample_id: &sample_id,
            content_hash: Some(content_hash),
            duration_seconds: decoded.duration_seconds,
            sr_used: decoded.sample_rate_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        &tx,
        &sample_id,
        &feature_blob,
        light_dsp_blob.as_deref(),
        rms,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
        computed_at,
    )?;
    db::upsert_cached_features(
        &tx,
        db::CachedFeaturesUpsert {
            content_hash,
            analysis_version,
            feat_version: wavecrate_analysis::vector::FEATURE_VERSION_V1,
            vec_blob: &feature_blob,
            light_dsp_blob: light_dsp_blob.as_deref(),
            rms,
            computed_at,
            duration_seconds: decoded.duration_seconds,
            sr_used: decoded.sample_rate_used,
        },
    )?;
    db::telemetry::commit_transaction(tx, "readiness_feature_publish")
        .map_err(|error| format!("Failed to commit readiness features: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
    checkpoint(cancel, "feature analysis cancelled after publication")?;
    Ok(true)
}

fn materialize_cached_features(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    sample_id: &str,
    content_hash: &str,
    analysis_version: &str,
    cached: &db::CachedFeatures,
) -> Result<bool, String> {
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_feature_cache_apply")
        .map_err(|error| format!("Failed to start readiness feature cache transaction: {error}"))?;
    if db::sample_content_hash(&tx, sample_id)?.as_deref() != Some(content_hash) {
        db::telemetry::commit_transaction(tx, "readiness_feature_cache_stale")
            .map_err(|error| format!("Failed to commit stale feature cache skip: {error}"))?;
        return Ok(false);
    }
    db::update_analysis_metadata(
        &tx,
        db::AnalysisMetadataUpdate {
            sample_id,
            content_hash: Some(content_hash),
            duration_seconds: cached.duration_seconds,
            sr_used: cached.sr_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        &tx,
        sample_id,
        &cached.vec_blob,
        cached.light_dsp_blob.as_deref(),
        cached.rms,
        cached.feat_version,
        cached.computed_at,
    )?;
    db::telemetry::commit_transaction(tx, "readiness_feature_cache_apply")
        .map_err(|error| format!("Failed to commit cached readiness features: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
    Ok(true)
}

fn ensure_current_sample_row(
    conn: &mut rusqlite::Connection,
    sample_id: &str,
    relative_path: &Path,
    content_hash: &str,
    source_root: &Path,
) -> Result<bool, String> {
    let relative_path = relative_path.to_string_lossy().replace('\\', "/");
    let manifest = conn
        .query_row(
            "SELECT file_size, modified_ns
             FROM wav_files
             WHERE path = ?1 AND content_hash = ?2 AND missing = 0",
            rusqlite::params![relative_path, content_hash],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| format!("Failed to read readiness sample manifest: {error}"))?;
    let Some((size, modified_ns)) = manifest else {
        return Ok(false);
    };
    let size = u64::try_from(size)
        .map_err(|_| format!("Readiness sample has a negative file size: {relative_path}"))?;
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_sample_upsert")
        .map_err(|error| format!("Failed to start readiness sample transaction: {error}"))?;
    db::upsert_samples_in_tx(
        &tx,
        &[db::SampleMetadata {
            sample_id: sample_id.to_string(),
            content_hash: content_hash.to_string(),
            size,
            mtime_ns: modified_ns,
        }],
    )?;
    db::telemetry::commit_transaction(tx, "readiness_sample_upsert")
        .map_err(|error| format!("Failed to commit readiness sample metadata: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
    Ok(true)
}

pub(crate) fn run_embedding_stage(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    checkpoint(cancel, "embedding analysis cancelled")?;
    let sample_id = db::build_sample_id(source_id, relative_path);
    if db::sample_content_hash(conn, &sample_id)?.as_deref() != Some(content_hash) {
        return Ok(false);
    }
    if db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )?
    .is_none()
    {
        return Ok(false);
    }
    let job = db::ClaimedJob {
        id: -1,
        sample_id: sample_id.clone(),
        content_hash: Some(
            serde_json::to_string(&[sample_id.as_str()]).map_err(|error| {
                format!("Failed to encode readiness embedding payload: {error}")
            })?,
        ),
        job_type: db::EMBEDDING_BACKFILL_JOB_TYPE.to_string(),
        source_root: source_root.to_path_buf(),
    };
    backfill::run_embedding_backfill_job_with_worker_limit(
        conn,
        &job,
        true,
        wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
        analysis_version,
        Some(cancel),
        Some(1),
    )?;
    checkpoint(cancel, "embedding analysis cancelled after publication")?;
    let embedding = db::cached_embedding_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
    )?;
    let aspects = db::cached_aspect_descriptors_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    )?;
    Ok(embedding.is_some() && aspects.is_some())
}

fn checkpoint(cancel: &AtomicBool, reason: &'static str) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err(reason.to_string())
    } else {
        Ok(())
    }
}
