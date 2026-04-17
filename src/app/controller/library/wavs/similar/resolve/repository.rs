//! Source/DB lookup helpers for similarity resolution.

use crate::app::controller::AppController;
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::SourceId;
use rusqlite::params_from_iter;
use std::collections::HashMap;

use super::super::FEATURE_RMS_INDEX;

const SQLITE_IN_BATCH_SIZE: usize = 900;

/// Resolve the sample identifier for one visible browser row.
pub(crate) fn resolve_sample_id_for_visible_row(
    controller: &mut AppController,
    visible_row: usize,
) -> Result<(String, usize), String> {
    let source_id = resolve_selected_source(controller)?;
    let entry_index = resolve_visible_row_index(controller, visible_row)?;
    let sample_id = resolve_sample_id_for_entry(controller, &source_id, entry_index)?;
    Ok((sample_id, entry_index))
}

fn resolve_selected_source(controller: &AppController) -> Result<SourceId, String> {
    controller
        .selection_state
        .ctx
        .selected_source
        .clone()
        .ok_or_else(|| "No active source selected".to_string())
}

fn resolve_visible_row_index(
    controller: &AppController,
    visible_row: usize,
) -> Result<usize, String> {
    controller
        .ui
        .browser
        .viewport
        .visible
        .get(visible_row)
        .ok_or_else(|| "Selected row is out of range".to_string())
}

fn resolve_sample_id_for_entry(
    controller: &mut AppController,
    source_id: &SourceId,
    entry_index: usize,
) -> Result<String, String> {
    let entry = controller
        .wav_entry(entry_index)
        .ok_or_else(|| "Sample entry missing".to_string())?;
    Ok(analysis_jobs::build_sample_id(
        source_id.as_str(),
        &entry.relative_path,
    ))
}

/// Open the selected source DB for similarity lookup.
pub(crate) fn open_source_db_for_id(
    controller: &AppController,
    source_id: &SourceId,
) -> Result<rusqlite::Connection, String> {
    let source = controller
        .library
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .ok_or_else(|| "Source not found".to_string())?;
    analysis_jobs::open_source_db(&source.root)
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
        let sql = format!(
            "SELECT sample_id, vec FROM embeddings
             WHERE model_id = ?1 AND sample_id IN ({})",
            placeholder_list(2, batch.len())
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|err| format!("Load embeddings failed: {err}"))?;
        let mut params = Vec::with_capacity(batch.len() + 1);
        params.push(rusqlite::types::Value::from(
            crate::analysis::similarity::SIMILARITY_MODEL_ID.to_string(),
        ));
        params.extend(batch.iter().cloned().map(rusqlite::types::Value::from));
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
            let SimilarityFeatureMetrics { light_dsp, rms } =
                decode_similarity_feature_metrics(&blob, feat_version)?;
            metrics.insert(sample_id, SimilarityFeatureMetrics { light_dsp, rms });
        }
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
    Ok(rms_by_sample)
}

fn decode_similarity_feature_metrics(
    blob: &[u8],
    feat_version: i64,
) -> Result<SimilarityFeatureMetrics, String> {
    if feat_version == crate::analysis::FEATURE_VERSION_V1 {
        let light_dsp = decode_f32_prefix(blob, crate::analysis::LIGHT_DSP_VECTOR_LEN)?;
        let rms = decode_feature_rms(blob, feat_version)?;
        return Ok(SimilarityFeatureMetrics {
            light_dsp: Some(super::normalize_l2(light_dsp)),
            rms,
        });
    }

    let features = crate::analysis::decode_f32_le_blob(blob)?;
    let rms = features.get(FEATURE_RMS_INDEX).copied();
    let light_dsp = crate::analysis::light_dsp_from_features_v1(&features).map(super::normalize_l2);
    Ok(SimilarityFeatureMetrics { light_dsp, rms })
}

fn decode_feature_rms(blob: &[u8], feat_version: i64) -> Result<Option<f32>, String> {
    if feat_version == crate::analysis::FEATURE_VERSION_V1 {
        return decode_f32_at(blob, FEATURE_RMS_INDEX).map(Some);
    }
    let features = crate::analysis::decode_f32_le_blob(blob)?;
    Ok(features.get(FEATURE_RMS_INDEX).copied())
}

fn decode_f32_prefix(blob: &[u8], count: usize) -> Result<Vec<f32>, String> {
    (0..count).map(|index| decode_f32_at(blob, index)).collect()
}

fn decode_f32_at(blob: &[u8], index: usize) -> Result<f32, String> {
    if !blob.len().is_multiple_of(4) {
        return Err("Feature blob length is not a multiple of 4 bytes".to_string());
    }
    let start = index.saturating_mul(4);
    let end = start.saturating_add(4);
    let Some(bytes) = blob.get(start..end) else {
        return Err(format!("Feature blob missing value at index {index}"));
    };
    Ok(f32::from_le_bytes(bytes.try_into().map_err(|_| {
        format!("Feature blob missing value at index {index}")
    })?))
}

fn placeholder_list(start_index: usize, count: usize) -> String {
    (0..count)
        .map(|offset| format!("?{}", start_index + offset))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Decoded feature metrics reused across similarity reranking stages.
pub(crate) struct SimilarityFeatureMetrics {
    /// Lightweight normalized DSP summary derived from the persisted feature blob.
    pub(crate) light_dsp: Option<Vec<f32>>,
    /// RMS feature used to skip effectively silent duplicate matches.
    pub(crate) rms: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::vector::encode_f32_le_blob;
    use crate::app::controller::test_support::dummy_controller;
    use crate::app::state::VisibleRows;
    use rusqlite::params;

    fn in_memory_similarity_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE embeddings (
                sample_id TEXT PRIMARY KEY,
                model_id TEXT NOT NULL,
                dim INTEGER NOT NULL,
                dtype TEXT NOT NULL,
                l2_normed INTEGER NOT NULL,
                vec BLOB NOT NULL,
                created_at INTEGER NOT NULL
             ) WITHOUT ROWID;
             CREATE TABLE features (
                sample_id TEXT PRIMARY KEY,
                feat_version INTEGER NOT NULL,
                vec_blob BLOB NOT NULL,
                computed_at INTEGER NOT NULL
             ) WITHOUT ROWID;",
        )
        .unwrap();
        conn
    }

    fn insert_embedding(conn: &rusqlite::Connection, sample_id: &str, values: &[f32]) {
        conn.execute(
            "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
            params![
                sample_id,
                crate::analysis::similarity::SIMILARITY_MODEL_ID,
                values.len() as i64,
                encode_f32_le_blob(values),
            ],
        )
        .unwrap();
    }

    fn insert_features(conn: &rusqlite::Connection, sample_id: &str, values: &[f32]) {
        let mut features = vec![0.0_f32; crate::analysis::FEATURE_VECTOR_LEN_V1];
        features[..values.len()].copy_from_slice(values);
        conn.execute(
            "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
             VALUES (?1, ?2, ?3, 0)",
            params![
                sample_id,
                crate::analysis::FEATURE_VERSION_V1,
                encode_f32_le_blob(&features),
            ],
        )
        .unwrap();
    }

    #[test]
    fn resolve_sample_id_for_visible_row_errors_on_empty_visible_rows() {
        let (mut controller, _source) = dummy_controller();
        controller.ui.browser.viewport.visible = VisibleRows::List(Vec::new().into());
        let err = resolve_sample_id_for_visible_row(&mut controller, 0).unwrap_err();
        assert_eq!(err, "Selected row is out of range");
    }

    #[test]
    fn resolve_sample_id_for_visible_row_errors_on_missing_entry() {
        let (mut controller, _source) = dummy_controller();
        controller.ui.browser.viewport.visible = VisibleRows::List(vec![0].into());
        let err = resolve_sample_id_for_visible_row(&mut controller, 0).unwrap_err();
        assert_eq!(err, "Sample entry missing");
    }

    #[test]
    fn batched_similarity_loaders_decode_embeddings_and_feature_metrics() {
        let conn = in_memory_similarity_conn();
        insert_embedding(&conn, "sample-a", &[1.0, 0.0, 0.0]);
        insert_embedding(&conn, "sample-b", &[0.0, 1.0, 0.0]);
        insert_features(&conn, "sample-a", &[0.9, 0.1, 0.25]);
        insert_features(&conn, "sample-b", &[0.2, 0.8, 0.5]);

        let sample_ids = vec!["sample-a".to_string(), "sample-b".to_string()];
        let embeddings = load_embeddings_for_samples(&conn, &sample_ids).unwrap();
        let metrics = load_feature_metrics_for_samples(&conn, &sample_ids).unwrap();

        assert_eq!(embeddings["sample-a"], vec![1.0, 0.0, 0.0]);
        assert_eq!(embeddings["sample-b"], vec![0.0, 1.0, 0.0]);
        assert_eq!(metrics["sample-a"].rms, Some(0.25));
        assert_eq!(metrics["sample-b"].rms, Some(0.5));
        assert!(metrics["sample-a"].light_dsp.is_some());
        assert!(metrics["sample-b"].light_dsp.is_some());
    }

    #[test]
    fn rms_loader_extracts_v1_rms_without_full_feature_decode() {
        let conn = in_memory_similarity_conn();
        insert_features(&conn, "sample-a", &[0.9, 0.1, 0.25]);
        insert_features(&conn, "sample-b", &[0.2, 0.8, 0.5]);

        let sample_ids = vec!["sample-a".to_string(), "sample-b".to_string()];
        let rms_by_sample = load_rms_for_samples(&conn, &sample_ids).unwrap();

        assert_eq!(rms_by_sample["sample-a"], 0.25);
        assert_eq!(rms_by_sample["sample-b"], 0.5);
    }

    #[test]
    fn rms_loader_falls_back_for_unknown_feature_versions() {
        let conn = in_memory_similarity_conn();
        let mut features = vec![0.0_f32; crate::analysis::FEATURE_VECTOR_LEN_V1];
        features[FEATURE_RMS_INDEX] = 0.75;
        conn.execute(
            "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
             VALUES (?1, ?2, ?3, 0)",
            params![
                "sample-a",
                crate::analysis::FEATURE_VERSION_V1 + 1,
                encode_f32_le_blob(&features),
            ],
        )
        .unwrap();

        let rms = load_rms_for_sample(&conn, "sample-a").unwrap();

        assert_eq!(rms, Some(0.75));
    }

    #[test]
    fn batched_similarity_loaders_span_sqlite_chunk_boundaries() {
        let conn = in_memory_similarity_conn();
        let sample_ids = (0..(SQLITE_IN_BATCH_SIZE + 5))
            .map(|index| format!("sample-{index}"))
            .collect::<Vec<_>>();
        for sample_id in &sample_ids {
            insert_embedding(&conn, sample_id, &[1.0, 0.0, 0.0]);
        }

        let embeddings = load_embeddings_for_samples(&conn, &sample_ids).unwrap();

        assert_eq!(embeddings.len(), sample_ids.len());
        assert_eq!(embeddings["sample-0"], vec![1.0, 0.0, 0.0]);
        assert_eq!(
            embeddings[&format!("sample-{}", SQLITE_IN_BATCH_SIZE + 4)],
            vec![1.0, 0.0, 0.0]
        );
    }
}
