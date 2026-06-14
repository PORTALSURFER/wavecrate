use super::super::decode::{decode_feature_rms, feature_metric_column_missing};
use super::batch::{SQLITE_IN_BATCH_SIZE, placeholder_list, sample_id_values};
use rusqlite::params_from_iter;
use std::collections::HashMap;

/// Load the RMS feature value used for duplicate/silence filtering.
pub(crate) fn load_rms_for_sample(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<f32>, String> {
    Ok(load_rms_for_samples(conn, &[sample_id.to_string()])?.remove(sample_id))
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
            if let Some(rms) = decode_feature_rms(&blob, feat_version)? {
                rms_by_sample.insert(sample_id, rms);
            }
        }
    }
    Ok(())
}
