use crate::app::controller::library::analysis_jobs::db;

use super::support::now_epoch_seconds;

pub(crate) fn apply_cached_features_and_embedding(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    content_hash: &str,
    features: &db::CachedFeatures,
    embedding: &db::CachedEmbedding,
    embedding_vec: &[f32],
    analysis_version: &str,
) -> Result<(), String> {
    db::update_analysis_metadata(
        conn,
        db::AnalysisMetadataUpdate {
            sample_id: &job.sample_id,
            content_hash: Some(content_hash),
            duration_seconds: features.duration_seconds,
            sr_used: features.sr_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        conn,
        &job.sample_id,
        &features.vec_blob,
        features.feat_version,
        features.computed_at,
    )?;
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
    crate::analysis::ann_index::upsert_embedding(conn, &job.sample_id, embedding_vec)?;
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
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    decoded: crate::analysis::audio::AnalysisAudio,
    analysis_version: &str,
    needs_embedding_upsert: bool,
    do_ann_upsert: bool,
) -> Result<(), String> {
    let content_hash = job
        .content_hash
        .as_deref()
        .ok_or_else(|| format!("Missing content_hash for analysis job {}", job.sample_id))?;
    let time_domain = crate::analysis::time_domain::extract_time_domain_features(
        &decoded.mono,
        decoded.sample_rate_used,
    );
    let frequency_domain = crate::analysis::frequency_domain::extract_frequency_domain_features(
        &decoded.mono,
        decoded.sample_rate_used,
    )?;
    let features =
        crate::analysis::features::AnalysisFeaturesV1::new(time_domain, frequency_domain);
    let vector = crate::analysis::vector::to_f32_vector_v1(&features);
    let embedding = crate::analysis::similarity::embedding_from_features(&vector)?;
    if needs_embedding_upsert {
        let embedding_blob = crate::analysis::vector::encode_f32_le_blob(&embedding);
        let created_at = now_epoch_seconds();
        db::upsert_embedding(
            conn,
            db::EmbeddingUpsert {
                sample_id: &job.sample_id,
                model_id: crate::analysis::similarity::SIMILARITY_MODEL_ID,
                dim: crate::analysis::similarity::SIMILARITY_DIM as i64,
                dtype: crate::analysis::similarity::SIMILARITY_DTYPE_F32,
                l2_normed: true,
                vec_blob: &embedding_blob,
                created_at,
            },
        )?;
    }
    db::update_analysis_metadata(
        conn,
        db::AnalysisMetadataUpdate {
            sample_id: &job.sample_id,
            content_hash: job.content_hash.as_deref(),
            duration_seconds: decoded.duration_seconds,
            sr_used: decoded.sample_rate_used,
            analysis_version,
        },
    )?;
    let current_hash = db::sample_content_hash(conn, &job.sample_id)?;
    if current_hash.as_deref() != Some(content_hash) {
        return Ok(());
    }
    if do_ann_upsert {
        crate::analysis::ann_index::upsert_embedding(conn, &job.sample_id, &embedding)?;
    }
    let blob = crate::analysis::vector::encode_f32_le_blob(&vector);
    let computed_at = now_epoch_seconds();
    db::upsert_analysis_features(
        conn,
        &job.sample_id,
        &blob,
        crate::analysis::vector::FEATURE_VERSION_V1,
        computed_at,
    )?;
    let embedding_blob = crate::analysis::vector::encode_f32_le_blob(&embedding);
    db::upsert_cached_features(
        conn,
        db::CachedFeaturesUpsert {
            content_hash,
            analysis_version,
            feat_version: crate::analysis::vector::FEATURE_VERSION_V1,
            vec_blob: &blob,
            computed_at,
            duration_seconds: decoded.duration_seconds,
            sr_used: decoded.sample_rate_used,
        },
    )?;
    db::upsert_cached_embedding(
        conn,
        db::CachedEmbeddingUpsert {
            content_hash,
            analysis_version,
            model_id: crate::analysis::similarity::SIMILARITY_MODEL_ID,
            dim: crate::analysis::similarity::SIMILARITY_DIM as i64,
            dtype: crate::analysis::similarity::SIMILARITY_DTYPE_F32,
            l2_normed: true,
            vec_blob: &embedding_blob,
            created_at: now_epoch_seconds(),
        },
    )?;
    Ok(())
}
