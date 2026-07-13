use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::{SampleSource, SourceDatabase, SourceId};
use std::collections::HashSet;

pub(crate) fn read_source_scan_timestamp(source: &SampleSource) -> Option<i64> {
    let db = SourceDatabase::open_for_background_job(&source.root).ok()?;
    db.get_metadata(crate::sample_sources::db::META_LAST_SCAN_COMPLETED_AT)
        .ok()
        .flatten()
        .and_then(|value| value.parse().ok())
}

pub(crate) fn read_source_prep_timestamp(source: &SampleSource) -> Option<i64> {
    let db = SourceDatabase::open_for_background_job(&source.root).ok()?;
    db.get_metadata(crate::sample_sources::db::META_LAST_SIMILARITY_PREP_SCAN_AT)
        .ok()
        .flatten()
        .and_then(|value| value.parse().ok())
}

/// Handles record similarity prep scan timestamp.
pub(crate) fn record_similarity_prep_scan_timestamp(
    source: &SampleSource,
    scan_completed_at: i64,
) -> Result<(), String> {
    let db =
        SourceDatabase::open_for_background_job(&source.root).map_err(|err| err.to_string())?;
    db.set_metadata(
        crate::sample_sources::db::META_LAST_SIMILARITY_PREP_SCAN_AT,
        &scan_completed_at.to_string(),
    )
    .map_err(|err| err.to_string())
}

pub(crate) fn source_has_embeddings(source: &SampleSource) -> bool {
    let Ok(sample_ids) = current_present_sample_ids(source) else {
        return false;
    };
    if sample_ids.is_empty() {
        return true;
    }
    let Ok(conn) = analysis_jobs::open_source_db_ui_read(&source.root) else {
        return false;
    };
    let model_id = wavecrate_analysis::similarity::SIMILARITY_MODEL_ID;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered_by_embeddings(&conn, model_id, &sample_id_prefix, &sample_ids)
        .unwrap_or(false)
}

pub(crate) fn source_has_layout(source: &SampleSource, umap_version: &str) -> bool {
    let Ok(sample_ids) = current_present_sample_ids(source) else {
        return false;
    };
    if sample_ids.is_empty() {
        return true;
    }
    let Ok(conn) = analysis_jobs::open_source_db_ui_read(&source.root) else {
        return false;
    };
    let model_id = wavecrate_analysis::similarity::SIMILARITY_MODEL_ID;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered_by_layout(
        &conn,
        model_id,
        umap_version,
        &sample_id_prefix,
        &sample_ids,
    )
    .unwrap_or(false)
}

pub(crate) fn source_has_aspect_descriptors(source: &SampleSource) -> bool {
    let Ok(sample_ids) = current_present_sample_ids(source) else {
        return false;
    };
    if sample_ids.is_empty() {
        return true;
    }
    let Ok(conn) = analysis_jobs::open_source_db_ui_read(&source.root) else {
        return false;
    };
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered_by_aspect_descriptors(&conn, &sample_id_prefix, &sample_ids).unwrap_or(false)
}

/// Handles current present sample ids.
fn current_present_sample_ids(source: &SampleSource) -> Result<Vec<String>, String> {
    let source_db =
        SourceDatabase::open_for_background_job(&source.root).map_err(|err| err.to_string())?;
    let entries = source_db.list_files().map_err(|err| err.to_string())?;
    Ok(entries
        .into_iter()
        .filter(|entry| !entry.missing)
        .map(|entry| analysis_jobs::build_sample_id(source.id.as_str(), &entry.relative_path))
        .collect())
}

/// Handles sample ids covered by embeddings.
fn sample_ids_covered_by_embeddings(
    conn: &rusqlite::Connection,
    model_id: &str,
    sample_id_prefix: &str,
    sample_ids: &[String],
) -> Result<bool, String> {
    let covered = covered_sample_ids(
        conn,
        "SELECT sample_id FROM embeddings WHERE model_id = ?1 AND sample_id LIKE ?2",
        rusqlite::params![model_id, sample_id_prefix],
        "Load embedding coverage failed",
    )?;
    Ok(sample_ids
        .iter()
        .all(|sample_id| covered.contains(sample_id)))
}

/// Handles sample ids covered by layout.
fn sample_ids_covered_by_layout(
    conn: &rusqlite::Connection,
    model_id: &str,
    umap_version: &str,
    sample_id_prefix: &str,
    sample_ids: &[String],
) -> Result<bool, String> {
    let covered = covered_sample_ids(
        conn,
        "SELECT sample_id FROM layout_umap
         WHERE model_id = ?1 AND umap_version = ?2 AND sample_id LIKE ?3",
        rusqlite::params![model_id, umap_version, sample_id_prefix],
        "Load layout coverage failed",
    )?;
    Ok(sample_ids
        .iter()
        .all(|sample_id| covered.contains(sample_id)))
}

fn sample_ids_covered_by_aspect_descriptors(
    conn: &rusqlite::Connection,
    sample_id_prefix: &str,
    sample_ids: &[String],
) -> Result<bool, String> {
    let covered = covered_sample_ids(
        conn,
        "SELECT sample_id FROM similarity_aspect_descriptors
         WHERE model_id = ?1
           AND dim = ?2
           AND dtype = ?3
           AND l2_normed = 1
           AND sample_id LIKE ?4",
        rusqlite::params![
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            sample_id_prefix,
        ],
        "Load aspect descriptor coverage failed",
    )?;
    Ok(sample_ids
        .iter()
        .all(|sample_id| covered.contains(sample_id)))
}

/// Handles covered sample ids.
fn covered_sample_ids<P>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
    context: &str,
) -> Result<HashSet<String>, String>
where
    P: rusqlite::Params,
{
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("{context}: {err}"))?;
    stmt.query_map(params, |row| row.get::<_, String>(0))
        .map_err(|err| format!("{context}: {err}"))?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|err| format!("{context}: {err}"))
}

pub(crate) fn count_umap_layout_rows(
    conn: &rusqlite::Connection,
    model_id: &str,
    umap_version: &str,
    sample_id_prefix: &str,
) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM layout_umap
         WHERE model_id = ?1 AND umap_version = ?2 AND sample_id LIKE ?3",
        rusqlite::params![model_id, umap_version, sample_id_prefix],
        |row| row.get(0),
    )
    .map_err(|err| format!("Count layout rows failed: {err}"))
}

pub(crate) fn open_source_db_for_similarity(
    source_id: &SourceId,
) -> Result<analysis_jobs::AnalysisJobSession, String> {
    let state = crate::sample_sources::library::load().map_err(|err| err.to_string())?;
    let source = state
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .ok_or_else(|| "Source not found for similarity prep".to_string())?;
    analysis_jobs::open_source_db(&source.root)
}

#[cfg(test)]
/// Contains focused regression coverage for this module.
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Defines umap version.
    const UMAP_VERSION: &str = "test-umap";

    #[test]
    /// Verifies source has embeddings requires current sample identity coverage.
    fn source_has_embeddings_requires_current_sample_identity_coverage() {
        let (_dir, source) = source_with_stale_similarity_rows();

        assert!(
            !source_has_embeddings(&source),
            "stale embedding rows with the same count must not satisfy current samples"
        );

        insert_embedding(&source, "current-a.wav");
        assert!(
            !source_has_embeddings(&source),
            "partial current embedding coverage is still incomplete"
        );

        insert_embedding(&source, "current-b.wav");
        assert!(source_has_embeddings(&source));
    }

    #[test]
    /// Verifies source has layout requires current sample identity coverage.
    fn source_has_layout_requires_current_sample_identity_coverage() {
        let (_dir, source) = source_with_stale_similarity_rows();

        assert!(
            !source_has_layout(&source, UMAP_VERSION),
            "one stale layout row must not satisfy current layout coverage"
        );

        insert_layout(&source, "current-a.wav");
        assert!(
            !source_has_layout(&source, UMAP_VERSION),
            "partial current layout coverage is still incomplete"
        );

        insert_layout(&source, "current-b.wav");
        assert!(source_has_layout(&source, UMAP_VERSION));
    }

    #[test]
    /// Verifies source has aspect descriptors requires current sample identity coverage.
    fn source_has_aspect_descriptors_requires_current_sample_identity_coverage() {
        let (_dir, source) = source_with_stale_similarity_rows();

        assert!(
            !source_has_aspect_descriptors(&source),
            "stale aspect rows must not satisfy current samples"
        );

        insert_aspect_descriptors(&source, "current-a.wav");
        assert!(
            !source_has_aspect_descriptors(&source),
            "partial current aspect coverage is still incomplete"
        );

        insert_aspect_descriptors(&source, "current-b.wav");
        assert!(source_has_aspect_descriptors(&source));
    }

    #[test]
    /// Handles record similarity prep scan timestamp returns source db errors.
    fn record_similarity_prep_scan_timestamp_returns_source_db_errors() {
        let dir = tempdir().unwrap();
        let root_file = dir.path().join("not-a-source-dir");
        std::fs::write(&root_file, b"file blocks source db directory").unwrap();
        let source = SampleSource::new(root_file);

        assert!(record_similarity_prep_scan_timestamp(&source, 123).is_err());
    }

    /// Handles source with stale similarity rows.
    fn source_with_stale_similarity_rows() -> (tempfile::TempDir, SampleSource) {
        let dir = tempdir().unwrap();
        let root = dir.path().join("source");
        std::fs::create_dir_all(&root).unwrap();
        let source = SampleSource::new(root);
        let db = SourceDatabase::open_for_source_write(&source.root).unwrap();
        db.upsert_file(std::path::Path::new("old-a.wav"), 1, 1)
            .unwrap();
        db.upsert_file(std::path::Path::new("old-b.wav"), 1, 1)
            .unwrap();
        insert_embedding(&source, "old-a.wav");
        insert_embedding(&source, "old-b.wav");
        insert_layout(&source, "old-a.wav");
        insert_aspect_descriptors(&source, "old-a.wav");
        insert_aspect_descriptors(&source, "old-b.wav");
        db.remove_file(std::path::Path::new("old-a.wav")).unwrap();
        db.remove_file(std::path::Path::new("old-b.wav")).unwrap();
        db.upsert_file(std::path::Path::new("current-a.wav"), 1, 1)
            .unwrap();
        db.upsert_file(std::path::Path::new("current-b.wav"), 1, 1)
            .unwrap();
        (dir, source)
    }

    /// Handles sample id.
    fn sample_id(source: &SampleSource, relative_path: &str) -> String {
        analysis_jobs::build_sample_id(source.id.as_str(), std::path::Path::new(relative_path))
    }

    /// Handles insert embedding.
    fn insert_embedding(source: &SampleSource, relative_path: &str) {
        let conn = analysis_jobs::open_source_db(&source.root).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO embeddings
             (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES (?1, ?2, 1, 'f32', 1, ?3, 0)",
            rusqlite::params![
                sample_id(source, relative_path),
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                1.0_f32.to_le_bytes().to_vec(),
            ],
        )
        .unwrap();
    }

    /// Handles insert layout.
    fn insert_layout(source: &SampleSource, relative_path: &str) {
        let conn = analysis_jobs::open_source_db(&source.root).unwrap();
        ensure_sample_row(&conn, &sample_id(source, relative_path));
        conn.execute(
            "INSERT OR REPLACE INTO layout_umap
             (sample_id, model_id, umap_version, x, y, created_at)
             VALUES (?1, ?2, ?3, 0.0, 0.0, 0)",
            rusqlite::params![
                sample_id(source, relative_path),
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                UMAP_VERSION,
            ],
        )
        .unwrap();
    }

    /// Handles insert aspect descriptors.
    fn insert_aspect_descriptors(source: &SampleSource, relative_path: &str) {
        let conn = analysis_jobs::open_source_db(&source.root).unwrap();
        ensure_sample_row(&conn, &sample_id(source, relative_path));
        conn.execute(
            "INSERT OR REPLACE INTO similarity_aspect_descriptors
             (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
             VALUES (?1, ?2, ?3, 'f32', 1, ?4, ?5, 0)",
            rusqlite::params![
                sample_id(source, relative_path),
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                wavecrate_analysis::aspects::all_aspect_mask() as i64,
                vec![0_u8; wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4],
            ],
        )
        .unwrap();
    }

    /// Handles ensure sample row.
    fn ensure_sample_row(conn: &rusqlite::Connection, sample_id: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO samples (sample_id, content_hash, size, mtime_ns)
             VALUES (?1, 'hash', 1, 1)",
            [sample_id],
        )
        .unwrap();
    }
}
