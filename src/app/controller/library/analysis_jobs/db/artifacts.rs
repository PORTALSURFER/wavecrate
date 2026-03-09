use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

pub(crate) struct CachedFeatures {
    pub(crate) feat_version: i64,
    pub(crate) vec_blob: Vec<u8>,
    pub(crate) computed_at: i64,
    pub(crate) duration_seconds: f32,
    pub(crate) sr_used: u32,
}

pub(crate) struct CachedEmbedding {
    pub(crate) model_id: String,
    pub(crate) dim: i64,
    pub(crate) dtype: String,
    pub(crate) l2_normed: bool,
    pub(crate) vec_blob: Vec<u8>,
    pub(crate) created_at: i64,
}

/// Typed inputs for updating duration/sample-rate metadata on an existing sample row.
pub(crate) struct AnalysisMetadataUpdate<'a> {
    pub(crate) sample_id: &'a str,
    pub(crate) content_hash: Option<&'a str>,
    pub(crate) duration_seconds: f32,
    pub(crate) sr_used: u32,
    pub(crate) analysis_version: &'a str,
}

/// Typed inputs for inserting or replacing one embedding row.
pub(crate) struct EmbeddingUpsert<'a> {
    pub(crate) sample_id: &'a str,
    pub(crate) model_id: &'a str,
    pub(crate) dim: i64,
    pub(crate) dtype: &'a str,
    pub(crate) l2_normed: bool,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) created_at: i64,
}

/// Typed inputs for caching reusable feature vectors by content hash.
pub(crate) struct CachedFeaturesUpsert<'a> {
    pub(crate) content_hash: &'a str,
    pub(crate) analysis_version: &'a str,
    pub(crate) feat_version: i64,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) computed_at: i64,
    pub(crate) duration_seconds: f32,
    pub(crate) sr_used: u32,
}

/// Typed inputs for caching reusable embeddings by content hash and model.
pub(crate) struct CachedEmbeddingUpsert<'a> {
    pub(crate) content_hash: &'a str,
    pub(crate) analysis_version: &'a str,
    pub(crate) model_id: &'a str,
    pub(crate) dim: i64,
    pub(crate) dtype: &'a str,
    pub(crate) l2_normed: bool,
    pub(crate) vec_blob: &'a [u8],
    pub(crate) created_at: i64,
}

pub(crate) fn invalidate_analysis_artifacts(
    conn: &mut Connection,
    sample_ids: &[String],
) -> Result<(), String> {
    if sample_ids.is_empty() {
        return Ok(());
    }
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Failed to start analysis invalidation transaction: {err}"))?;
    let mut stmt_features = tx
        .prepare("DELETE FROM features WHERE sample_id = ?1")
        .map_err(|err| format!("Failed to prepare analysis invalidation statement: {err}"))?;
    let mut stmt_embeddings = tx
        .prepare("DELETE FROM embeddings WHERE sample_id = ?1")
        .map_err(|err| format!("Failed to prepare analysis invalidation statement: {err}"))?;
    let mut stmt_legacy_features = tx
        .prepare("DELETE FROM analysis_features WHERE sample_id = ?1")
        .map_err(|err| format!("Failed to prepare analysis invalidation statement: {err}"))?;
    for sample_id in sample_ids {
        stmt_features
            .execute(params![sample_id])
            .map_err(|err| format!("Failed to invalidate analysis features: {err}"))?;
        stmt_embeddings
            .execute(params![sample_id])
            .map_err(|err| format!("Failed to invalidate embeddings: {err}"))?;
        stmt_legacy_features
            .execute(params![sample_id])
            .map_err(|err| format!("Failed to invalidate analysis features: {err}"))?;
    }
    drop(stmt_features);
    drop(stmt_embeddings);
    drop(stmt_legacy_features);
    tx.commit()
        .map_err(|err| format!("Failed to commit analysis invalidation transaction: {err}"))?;
    Ok(())
}

pub(crate) fn update_analysis_metadata(
    conn: &Connection,
    update: AnalysisMetadataUpdate<'_>,
) -> Result<(), String> {
    let updated = conn
        .execute(
            "UPDATE samples
             SET duration_seconds = ?3, sr_used = ?4, analysis_version = ?5
             WHERE sample_id = ?1 AND content_hash = COALESCE(?2, content_hash)",
            params![
                update.sample_id,
                update.content_hash,
                update.duration_seconds as f64,
                update.sr_used as i64,
                update.analysis_version
            ],
        )
        .map_err(|err| format!("Failed to update analysis metadata: {err}"))?;
    if updated == 0 {
        return Err(format!(
            "No sample row updated for sample_id={}",
            update.sample_id
        ));
    }
    Ok(())
}

/// Update duration/sample rate metadata without changing analysis version.
/// Returns true when the duration was updated.
pub(crate) fn update_sample_duration(
    conn: &Connection,
    sample_id: &str,
    duration_seconds: f32,
    sr_used: u32,
) -> Result<bool, String> {
    let updated = conn
        .execute(
            "UPDATE samples
             SET duration_seconds = ?2, sr_used = ?3
             WHERE sample_id = ?1
               AND (duration_seconds IS NULL OR duration_seconds <= 0)",
            params![sample_id, duration_seconds as f64, sr_used as i64],
        )
        .map_err(|err| format!("Failed to update sample duration: {err}"))?;
    Ok(updated > 0)
}

/// Persist the long-sample marker for a sample row.
pub(crate) fn update_sample_long_mark(
    conn: &Connection,
    sample_id: &str,
    long_sample_mark: bool,
) -> Result<(), String> {
    let mark = if long_sample_mark { 1i64 } else { 0i64 };
    conn.execute(
        "UPDATE samples SET long_sample_mark = ?2 WHERE sample_id = ?1",
        params![sample_id, mark],
    )
    .map_err(|err| format!("Failed to update long sample mark: {err}"))?;
    Ok(())
}

pub(crate) fn upsert_analysis_features(
    conn: &Connection,
    sample_id: &str,
    vec_blob: &[u8],
    feat_version: i64,
    computed_at: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(sample_id) DO UPDATE SET
            feat_version = excluded.feat_version,
            vec_blob = excluded.vec_blob,
            computed_at = excluded.computed_at",
        params![sample_id, feat_version, vec_blob, computed_at],
    )
    .map_err(|err| format!("Failed to upsert analysis features: {err}"))?;
    Ok(())
}

pub(crate) fn upsert_embedding(
    conn: &Connection,
    embedding: EmbeddingUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(sample_id) DO UPDATE SET
            model_id = excluded.model_id,
            dim = excluded.dim,
            dtype = excluded.dtype,
            l2_normed = excluded.l2_normed,
            vec = excluded.vec,
            created_at = excluded.created_at",
        params![
            embedding.sample_id,
            embedding.model_id,
            embedding.dim,
            embedding.dtype,
            embedding.l2_normed,
            embedding.vec_blob,
            embedding.created_at
        ],
    )
    .map_err(|err| format!("Failed to upsert embedding: {err}"))?;
    Ok(())
}

pub(crate) fn cached_features_by_hash(
    conn: &Connection,
    content_hash: &str,
    analysis_version: &str,
    feat_version: i64,
) -> Result<Option<CachedFeatures>, String> {
    conn.query_row(
        "SELECT feat_version, vec_blob, computed_at, duration_seconds, sr_used
         FROM analysis_cache_features
         WHERE content_hash = ?1 AND analysis_version = ?2 AND feat_version = ?3",
        params![content_hash, analysis_version, feat_version],
        |row| {
            Ok(CachedFeatures {
                feat_version: row.get(0)?,
                vec_blob: row.get(1)?,
                computed_at: row.get(2)?,
                duration_seconds: row.get::<_, f64>(3)? as f32,
                sr_used: row.get::<_, i64>(4)? as u32,
            })
        },
    )
    .optional()
    .map_err(|err| format!("Failed to load cached features for {content_hash}: {err}"))
}

pub(crate) fn cached_embedding_by_hash(
    conn: &Connection,
    content_hash: &str,
    analysis_version: &str,
    model_id: &str,
) -> Result<Option<CachedEmbedding>, String> {
    conn.query_row(
        "SELECT model_id, dim, dtype, l2_normed, vec, created_at
         FROM analysis_cache_embeddings
         WHERE content_hash = ?1 AND analysis_version = ?2 AND model_id = ?3",
        params![content_hash, analysis_version, model_id],
        |row| {
            Ok(CachedEmbedding {
                model_id: row.get(0)?,
                dim: row.get(1)?,
                dtype: row.get(2)?,
                l2_normed: row.get::<_, i64>(3)? != 0,
                vec_blob: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("Failed to load cached embedding for {content_hash}: {err}"))
}

pub(crate) fn upsert_cached_features(
    conn: &Connection,
    features: CachedFeaturesUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO analysis_cache_features
            (content_hash, analysis_version, feat_version, vec_blob, computed_at, duration_seconds, sr_used)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(content_hash) DO UPDATE SET
            analysis_version = excluded.analysis_version,
            feat_version = excluded.feat_version,
            vec_blob = excluded.vec_blob,
            computed_at = excluded.computed_at,
            duration_seconds = excluded.duration_seconds,
            sr_used = excluded.sr_used",
        params![
            features.content_hash,
            features.analysis_version,
            features.feat_version,
            features.vec_blob,
            features.computed_at,
            features.duration_seconds as f64,
            features.sr_used as i64
        ],
    )
    .map_err(|err| format!("Failed to upsert cached features: {err}"))?;
    Ok(())
}

pub(crate) fn upsert_cached_embedding(
    conn: &Connection,
    embedding: CachedEmbeddingUpsert<'_>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO analysis_cache_embeddings
            (content_hash, analysis_version, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(content_hash, model_id) DO UPDATE SET
            analysis_version = excluded.analysis_version,
            dim = excluded.dim,
            dtype = excluded.dtype,
            l2_normed = excluded.l2_normed,
            vec = excluded.vec,
            created_at = excluded.created_at",
        params![
            embedding.content_hash,
            embedding.analysis_version,
            embedding.model_id,
            embedding.dim,
            embedding.dtype,
            embedding.l2_normed,
            embedding.vec_blob,
            embedding.created_at
        ],
    )
    .map_err(|err| format!("Failed to upsert cached embedding: {err}"))?;
    Ok(())
}
