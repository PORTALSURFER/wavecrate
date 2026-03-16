//! Embedding load and validation helpers for the HDBSCAN helper binary.

use rusqlite::{Connection, params};

/// Load one model's embeddings in stable sample order and validate their dimensions.
pub(super) fn load_embeddings(
    conn: &Connection,
    model_id: &str,
) -> Result<(Vec<String>, Vec<Vec<f32>>), String> {
    let mut stmt = conn
        .prepare(
            "SELECT sample_id, dim, vec
             FROM embeddings
             WHERE model_id = ?1
             ORDER BY sample_id ASC",
        )
        .map_err(|err| format!("Prepare embedding query failed: {err}"))?;
    let rows = stmt
        .query_map(params![model_id], |row| {
            let sample_id: String = row.get(0)?;
            let dim: i64 = row.get(1)?;
            let blob: Vec<u8> = row.get(2)?;
            Ok((sample_id, dim as usize, blob))
        })
        .map_err(|err| format!("Query embeddings failed: {err}"))?;
    let mut sample_ids = Vec::new();
    let mut data = Vec::new();
    let mut expected_dim: Option<usize> = None;
    for row in rows {
        let (sample_id, dim, blob) =
            row.map_err(|err| format!("Read embedding row failed: {err}"))?;
        let vector = sempal::analysis::decode_f32_le_blob(&blob)?;
        validate_embedding_dim(&sample_id, dim, vector.len(), expected_dim)?;
        expected_dim.get_or_insert(dim);
        sample_ids.push(sample_id);
        data.push(vector);
    }
    Ok((sample_ids, data))
}

fn validate_embedding_dim(
    sample_id: &str,
    dim: usize,
    actual_len: usize,
    expected_dim: Option<usize>,
) -> Result<(), String> {
    if actual_len != dim {
        return Err(format!(
            "Embedding dim mismatch for {sample_id}: expected {dim}, got {actual_len}"
        ));
    }
    if let Some(expected) = expected_dim
        && dim != expected
    {
        return Err(format!(
            "Embedding dim mismatch: expected {expected}, got {dim} for {sample_id}"
        ));
    }
    Ok(())
}
