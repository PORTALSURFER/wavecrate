//! Similarity-map embedding load and persistence helpers.

use crate::analysis::decode_f32_le_blob;
use rusqlite::{Connection, params};
use std::time::{SystemTime, UNIX_EPOCH};

use super::LayoutPoint;

struct LoadedEmbeddingRow {
    sample_id: String,
    dim: usize,
    blob: Vec<u8>,
}

/// Load model embeddings from SQLite into one dense row-major vector matrix.
pub(super) fn load_embeddings(
    conn: &Connection,
    model_id: &str,
) -> Result<(Vec<String>, Vec<f64>, usize), String> {
    let count = count_embeddings(conn, model_id)?;
    let rows = load_embedding_rows(conn, model_id)?;
    collect_embeddings(rows, count)
}

/// Persist one projected layout into the legacy `layout_umap` compatibility table.
pub(super) fn write_layout(
    conn: &mut Connection,
    sample_ids: &[String],
    layout: &[LayoutPoint],
    model_id: &str,
    umap_version: &str,
) -> Result<usize, String> {
    let now = current_unix_timestamp()?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("Start transaction failed: {err}"))?;
    let mut stmt = tx
        .prepare(
            "INSERT INTO layout_umap (sample_id, model_id, umap_version, x, y, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(sample_id) DO UPDATE SET
                model_id = excluded.model_id,
                umap_version = excluded.umap_version,
                x = excluded.x,
                y = excluded.y,
                created_at = excluded.created_at",
        )
        .map_err(|err| format!("Prepare layout insert failed: {err}"))?;
    for (sample_id, coords) in sample_ids.iter().zip(layout.iter()) {
        stmt.execute(params![
            sample_id,
            model_id,
            umap_version,
            coords[0] as f64,
            coords[1] as f64,
            now
        ])
        .map_err(|err| format!("Insert layout failed: {err}"))?;
    }
    drop(stmt);
    tx.commit()
        .map_err(|err| format!("Commit layout failed: {err}"))?;
    Ok(sample_ids.len())
}

fn count_embeddings(conn: &Connection, model_id: &str) -> Result<usize, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM embeddings WHERE model_id = ?1",
        params![model_id],
        |row| row.get(0),
    )
    .map_err(|err| format!("Count embeddings failed: {err}"))
}

fn load_embedding_rows(
    conn: &Connection,
    model_id: &str,
) -> Result<Vec<LoadedEmbeddingRow>, String> {
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
            Ok(LoadedEmbeddingRow {
                sample_id,
                dim: dim as usize,
                blob,
            })
        })
        .map_err(|err| format!("Query embeddings failed: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Read embedding row failed: {err}"))
}

fn collect_embeddings(
    rows: Vec<LoadedEmbeddingRow>,
    count: usize,
) -> Result<(Vec<String>, Vec<f64>, usize), String> {
    let mut sample_ids = Vec::with_capacity(count);
    let mut vectors = Vec::new();
    let mut expected_dim = None;
    for row in rows {
        let dim = resolve_expected_dim(expected_dim, row.dim, &row.sample_id)?;
        if expected_dim.is_none() {
            vectors.reserve_exact(count * dim);
            expected_dim = Some(dim);
        }
        append_embedding_vector(&mut sample_ids, &mut vectors, row)?;
    }
    let dim = expected_dim.unwrap_or(0);
    if dim == 0 {
        return Err("No embeddings found for model".to_string());
    }
    Ok((sample_ids, vectors, dim))
}

fn resolve_expected_dim(
    expected_dim: Option<usize>,
    dim: usize,
    sample_id: &str,
) -> Result<usize, String> {
    if let Some(expected) = expected_dim {
        if dim != expected {
            return Err(format!(
                "Embedding dim mismatch: expected {expected}, got {dim} for {sample_id}"
            ));
        }
    }
    Ok(expected_dim.unwrap_or(dim))
}

fn append_embedding_vector(
    sample_ids: &mut Vec<String>,
    vectors: &mut Vec<f64>,
    row: LoadedEmbeddingRow,
) -> Result<(), String> {
    let vec = decode_f32_le_blob(&row.blob)?;
    if vec.len() != row.dim {
        return Err(format!(
            "Embedding dim mismatch for {}: expected {}, got {}",
            row.sample_id,
            row.dim,
            vec.len()
        ));
    }
    sample_ids.push(row.sample_id);
    vectors.extend(vec.into_iter().map(f64::from));
    Ok(())
}

fn current_unix_timestamp() -> Result<i64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .map_err(|_| "Invalid system time".to_string())
}
