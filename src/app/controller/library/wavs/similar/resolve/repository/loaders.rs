//! Batched SQLite loaders for similarity embeddings and feature metrics.

use rusqlite::params_from_iter;
use std::collections::HashMap;

use super::decode::{
    decode_feature_rms, decode_light_dsp_blob, decode_similarity_feature_metrics,
    feature_metric_column_missing, placeholder_list,
};

pub(super) const SQLITE_IN_BATCH_SIZE: usize = 900;

/// Decoded feature metrics reused across similarity reranking stages.
#[derive(Default)]
pub(crate) struct SimilarityFeatureMetrics {
    /// Lightweight normalized DSP summary derived from the persisted feature blob.
    pub(crate) light_dsp: Option<Vec<f32>>,
    /// RMS feature used to skip effectively silent duplicate matches.
    pub(crate) rms: Option<f32>,
}

/// Query-sample vectors reused across similarity resolution stages.
pub(crate) struct QuerySimilarityInputs {
    /// Normalized embedding used for ANN reranking.
    pub(crate) embedding: Option<Vec<f32>>,
    /// Lightweight normalized DSP vector used for the DSP blend path.
    pub(crate) light_dsp: Option<Vec<f32>>,
    /// RMS feature used for duplicate/silence filtering.
    pub(crate) rms: Option<f32>,
}

/// Load the lightweight DSP vector used to refine ANN similarity results.
pub(crate) fn load_light_dsp_for_sample(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<Vec<f32>>, String> {
    Ok(
        load_feature_metrics_for_samples(conn, &[sample_id.to_string()])?
            .remove(sample_id)
            .and_then(|metrics| metrics.light_dsp),
    )
}

/// Load the RMS feature value used for duplicate/silence filtering.
pub(crate) fn load_rms_for_sample(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<f32>, String> {
    Ok(load_rms_for_samples(conn, &[sample_id.to_string()])?.remove(sample_id))
}

/// Load the persisted similarity embedding for one sample.
pub(crate) fn load_embedding_for_sample(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<Vec<f32>>, String> {
    Ok(load_embeddings_for_samples(conn, &[sample_id.to_string()])?.remove(sample_id))
}

/// Load normalized similarity embeddings for a candidate set in one query.
pub(crate) fn load_embeddings_for_samples(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
) -> Result<HashMap<String, Vec<f32>>, String> {
    if sample_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut embeddings = HashMap::with_capacity(sample_ids.len());
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        load_embedding_batch(conn, batch, &mut embeddings)?;
    }
    Ok(embeddings)
}

/// Load lightweight DSP and RMS metrics for a candidate set in one feature query.
pub(crate) fn load_feature_metrics_for_samples(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
) -> Result<HashMap<String, SimilarityFeatureMetrics>, String> {
    if sample_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut metrics = HashMap::with_capacity(sample_ids.len());
    let mut fallback_ids = Vec::new();
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        fallback_ids.extend(load_persisted_feature_metrics_batch(
            conn,
            batch,
            &mut metrics,
        )?);
    }
    if !fallback_ids.is_empty() {
        load_feature_metrics_from_blob(conn, &fallback_ids, &mut metrics)?;
    }
    Ok(metrics)
}

/// Load only RMS feature values for a candidate set in one feature query.
pub(crate) fn load_rms_for_samples(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
) -> Result<HashMap<String, f32>, String> {
    if sample_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut rms_by_sample = HashMap::with_capacity(sample_ids.len());
    let mut fallback_ids = Vec::new();
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        fallback_ids.extend(load_persisted_rms_batch(conn, batch, &mut rms_by_sample)?);
    }
    if !fallback_ids.is_empty() {
        load_rms_from_blob(conn, &fallback_ids, &mut rms_by_sample)?;
    }
    Ok(rms_by_sample)
}

/// Load embedding plus lightweight feature metrics for one query sample with one feature-row lookup.
pub(crate) fn load_query_similarity_inputs(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<QuerySimilarityInputs, String> {
    let sample_ids = [sample_id.to_string()];
    let mut embeddings = load_embeddings_for_samples(conn, &sample_ids)?;
    let mut feature_metrics = load_feature_metrics_for_samples(conn, &sample_ids)?;
    let metrics = feature_metrics.remove(sample_id).unwrap_or_default();
    Ok(QuerySimilarityInputs {
        embedding: embeddings.remove(sample_id),
        light_dsp: metrics.light_dsp,
        rms: metrics.rms,
    })
}

fn load_embedding_batch(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    embeddings: &mut HashMap<String, Vec<f32>>,
) -> Result<(), String> {
    let sql = format!(
        "SELECT sample_id, vec FROM embeddings
         WHERE model_id = ?1 AND sample_id IN ({})",
        placeholder_list(2, sample_ids.len())
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Load embeddings failed: {err}"))?;
    let mut params = Vec::with_capacity(sample_ids.len() + 1);
    params.push(rusqlite::types::Value::from(
        crate::analysis::similarity::SIMILARITY_MODEL_ID.to_string(),
    ));
    params.extend(sample_ids.iter().cloned().map(rusqlite::types::Value::from));
    let mut rows = stmt
        .query(params_from_iter(params))
        .map_err(|err| format!("Load embeddings failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Load embeddings failed: {err}"))?
    {
        let sample_id = row
            .get::<_, String>(0)
            .map_err(|err| format!("Load embeddings failed: {err}"))?;
        let blob = row
            .get::<_, Vec<u8>>(1)
            .map_err(|err| format!("Load embeddings failed: {err}"))?;
        let embedding = crate::analysis::decode_f32_le_blob(&blob)?;
        embeddings.insert(sample_id, embedding);
    }
    Ok(())
}

fn load_persisted_feature_metrics_batch(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    metrics: &mut HashMap<String, SimilarityFeatureMetrics>,
) -> Result<Vec<String>, String> {
    let sql = format!(
        "SELECT sample_id, light_dsp_blob, rms FROM features WHERE sample_id IN ({})",
        placeholder_list(1, sample_ids.len())
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(err) if feature_metric_column_missing(&err) => return Ok(sample_ids.to_vec()),
        Err(err) => return Err(format!("Load features failed: {err}")),
    };
    let params = sample_ids
        .iter()
        .cloned()
        .map(rusqlite::types::Value::from)
        .collect::<Vec<_>>();
    let mut rows = stmt
        .query(params_from_iter(params))
        .map_err(|err| format!("Load features failed: {err}"))?;
    let mut fallback_ids = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Load features failed: {err}"))?
    {
        let sample_id = row
            .get::<_, String>(0)
            .map_err(|err| format!("Load features failed: {err}"))?;
        let light_dsp = row
            .get::<_, Option<Vec<u8>>>(1)
            .map_err(|err| format!("Load features failed: {err}"))?
            .map(|blob| decode_light_dsp_blob(&blob))
            .transpose()?;
        let rms = row
            .get::<_, Option<f64>>(2)
            .map_err(|err| format!("Load features failed: {err}"))?
            .map(|value| value as f32);
        if light_dsp.is_none() || rms.is_none() {
            fallback_ids.push(sample_id);
            continue;
        }
        metrics.insert(sample_id, SimilarityFeatureMetrics { light_dsp, rms });
    }
    Ok(fallback_ids)
}

fn load_feature_metrics_from_blob(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    metrics: &mut HashMap<String, SimilarityFeatureMetrics>,
) -> Result<(), String> {
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        let sql = format!(
            "SELECT sample_id, feat_version, vec_blob FROM features WHERE sample_id IN ({})",
            placeholder_list(1, batch.len())
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|err| format!("Load features failed: {err}"))?;
        let params = batch
            .iter()
            .cloned()
            .map(rusqlite::types::Value::from)
            .collect::<Vec<_>>();
        let mut rows = stmt
            .query(params_from_iter(params))
            .map_err(|err| format!("Load features failed: {err}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("Load features failed: {err}"))?
        {
            let sample_id = row
                .get::<_, String>(0)
                .map_err(|err| format!("Load features failed: {err}"))?;
            let feat_version = row
                .get::<_, i64>(1)
                .map_err(|err| format!("Load features failed: {err}"))?;
            let blob = row
                .get::<_, Vec<u8>>(2)
                .map_err(|err| format!("Load features failed: {err}"))?;
            metrics.insert(
                sample_id,
                decode_similarity_feature_metrics(&blob, feat_version)?,
            );
        }
    }
    Ok(())
}

fn load_persisted_rms_batch(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    rms_by_sample: &mut HashMap<String, f32>,
) -> Result<Vec<String>, String> {
    let sql = format!(
        "SELECT sample_id, rms FROM features WHERE sample_id IN ({})",
        placeholder_list(1, sample_ids.len())
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(err) if feature_metric_column_missing(&err) => return Ok(sample_ids.to_vec()),
        Err(err) => return Err(format!("Load features failed: {err}")),
    };
    let params = sample_ids
        .iter()
        .cloned()
        .map(rusqlite::types::Value::from)
        .collect::<Vec<_>>();
    let mut rows = stmt
        .query(params_from_iter(params))
        .map_err(|err| format!("Load features failed: {err}"))?;
    let mut fallback_ids = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Load features failed: {err}"))?
    {
        let sample_id = row
            .get::<_, String>(0)
            .map_err(|err| format!("Load features failed: {err}"))?;
        let rms = row
            .get::<_, Option<f64>>(1)
            .map_err(|err| format!("Load features failed: {err}"))?
            .map(|value| value as f32);
        if let Some(rms) = rms {
            rms_by_sample.insert(sample_id, rms);
        } else {
            fallback_ids.push(sample_id);
        }
    }
    Ok(fallback_ids)
}

fn load_rms_from_blob(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    rms_by_sample: &mut HashMap<String, f32>,
) -> Result<(), String> {
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        let sql = format!(
            "SELECT sample_id, feat_version, vec_blob FROM features WHERE sample_id IN ({})",
            placeholder_list(1, batch.len())
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|err| format!("Load features failed: {err}"))?;
        let params = batch
            .iter()
            .cloned()
            .map(rusqlite::types::Value::from)
            .collect::<Vec<_>>();
        let mut rows = stmt
            .query(params_from_iter(params))
            .map_err(|err| format!("Load features failed: {err}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("Load features failed: {err}"))?
        {
            let sample_id = row
                .get::<_, String>(0)
                .map_err(|err| format!("Load features failed: {err}"))?;
            let feat_version = row
                .get::<_, i64>(1)
                .map_err(|err| format!("Load features failed: {err}"))?;
            let blob = row
                .get::<_, Vec<u8>>(2)
                .map_err(|err| format!("Load features failed: {err}"))?;
            if let Some(rms) = decode_feature_rms(&blob, feat_version)? {
                rms_by_sample.insert(sample_id, rms);
            }
        }
    }
    Ok(())
}
