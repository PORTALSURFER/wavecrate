use super::batch::{SQLITE_IN_BATCH_SIZE, embedding_query_values, placeholder_list};
use rusqlite::params_from_iter;
use std::collections::HashMap;

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
    let mut rows = stmt
        .query(params_from_iter(embedding_query_values(sample_ids)))
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
