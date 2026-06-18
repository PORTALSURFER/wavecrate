use super::db;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) fn failed_samples_for_source(
    source: &crate::sample_sources::SampleSource,
) -> Result<HashMap<PathBuf, String>, String> {
    let conn = db::open_source_db(&source.root)?;
    failed_samples_for_source_conn(&conn, &source.id)
}

fn failed_samples_for_source_conn(
    conn: &Connection,
    source_id: &crate::sample_sources::SourceId,
) -> Result<HashMap<PathBuf, String>, String> {
    let embedding_model = wavecrate_analysis::similarity::SIMILARITY_MODEL_ID;
    let aspect_model = wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID;
    let analysis_version = wavecrate_analysis::analysis_version();
    let mut stmt = conn
        .prepare(
            "SELECT aj.relative_path, aj.last_error
             FROM analysis_jobs aj
             LEFT JOIN samples s ON s.sample_id = aj.sample_id
             LEFT JOIN features f
                ON f.sample_id = aj.sample_id AND f.feat_version = ?2
             LEFT JOIN embeddings e
                ON e.sample_id = aj.sample_id AND e.model_id = ?3
             LEFT JOIN similarity_aspect_descriptors a
                ON a.sample_id = aj.sample_id
               AND a.model_id = ?5
               AND a.dim = ?6
               AND a.dtype = ?7
               AND a.l2_normed = 1
             WHERE aj.status = 'failed'
               AND aj.source_id = ?1
               AND (
                  f.sample_id IS NULL
                  OR s.analysis_version IS NULL
                  OR s.analysis_version != ?4
                  OR e.sample_id IS NULL
                  OR a.sample_id IS NULL
               )
             ORDER BY aj.relative_path ASC",
        )
        .map_err(|err| format!("Failed to query failed analysis jobs: {err}"))?;
    let mut out = HashMap::new();
    let rows = stmt
        .query_map(
            params![
                source_id.as_str(),
                1i64,
                embedding_model,
                analysis_version,
                aspect_model,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            ],
            |row| {
                let relative_path: String = row.get(0)?;
                let last_error: Option<String> = row.get(1)?;
                Ok((relative_path, last_error))
            },
        )
        .map_err(|err| format!("Failed to query failed analysis jobs: {err}"))?;
    for row in rows {
        let (relative_path, last_error) =
            row.map_err(|err| format!("Failed to decode failed analysis job row: {err}"))?;
        out.insert(
            PathBuf::from(relative_path),
            last_error.unwrap_or_else(|| "Analysis failed".to_string()),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_dirs::ConfigBaseGuard;
    use tempfile::tempdir;

    #[test]
    fn loads_failed_jobs_for_source() {
        let config_dir = tempdir().unwrap();
        let _guard = ConfigBaseGuard::set(config_dir.path().to_path_buf());
        let source_root = tempdir().unwrap();
        let source = crate::sample_sources::SampleSource::new_with_id(
            crate::sample_sources::SourceId::from_string("s1"),
            source_root.path().to_path_buf(),
        );
        let conn = db::open_source_db(&source.root).unwrap();
        conn.execute_batch(
            "DELETE FROM analysis_jobs;
             DELETE FROM samples;
             DELETE FROM features;
             DELETE FROM embeddings;
             DELETE FROM similarity_aspect_descriptors;",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at, last_error)
             VALUES ('s1::Pack/a.wav', 's1', 'Pack/a.wav', 'x', 'failed', 1, 0, 'boom')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at)
             VALUES ('s1::Pack/b.wav', 's1', 'Pack/b.wav', 'x', 'failed', 1, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at, last_error)
             VALUES ('s2::Other/c.wav', 's2', 'Other/c.wav', 'x', 'failed', 1, 0, 'nope')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, analysis_version)
             VALUES ('s1::Pack/a.wav', 'h1', 1, 1, ?1)",
            params![wavecrate_analysis::analysis_version()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
             VALUES ('s1::Pack/a.wav', 1, X'00', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES ('s1::Pack/a.wav', ?1, 1, 'f32', 1, X'00', 0)",
            params![wavecrate_analysis::similarity::SIMILARITY_MODEL_ID],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO similarity_aspect_descriptors
             (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
             VALUES ('s1::Pack/a.wav', ?1, ?2, ?3, 1, ?4, ?5, 0)",
            params![
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
                wavecrate_analysis::aspects::all_aspect_mask() as i64,
                vec![0_u8; wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4],
            ],
        )
        .unwrap();

        let map = failed_samples_for_source_conn(&conn, &source.id).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get(&PathBuf::from("Pack/b.wav")).map(|s| s.as_str()),
            Some("Analysis failed")
        );
    }
}
