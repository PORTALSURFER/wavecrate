use super::super::decode::{
    decode_light_dsp_blob, decode_similarity_feature_metrics, feature_metric_column_missing,
};
use super::batch::{SQLITE_IN_BATCH_SIZE, placeholder_list, sample_id_values};
use rusqlite::params_from_iter;
use std::collections::HashMap;

/// Decoded feature metrics reused across similarity reranking stages.
#[derive(Default)]
pub(crate) struct SimilarityFeatureMetrics {
    /// Lightweight normalized DSP summary derived from the persisted feature blob.
    pub(crate) light_dsp: Option<Vec<f32>>,
    /// RMS feature used to skip effectively silent duplicate matches.
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
    let mut rows = stmt
        .query(params_from_iter(sample_id_values(sample_ids)))
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
        let mut rows = stmt
            .query(params_from_iter(sample_id_values(batch)))
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
