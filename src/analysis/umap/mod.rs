//! Legacy UMAP-named similarity-map layout facade.
//!
//! The current implementation projects embeddings with t-SNE while keeping the
//! established `umap` naming and `layout_umap` persistence schema for
//! compatibility with existing callers, reports, and stored rows.

mod projection;
mod report;
mod storage;

use rusqlite::Connection;
use std::path::Path;

use projection::compute_tsne;
use report::validate_layout;
pub use report::{MapLayoutReport, UmapReport, default_layout_report_path, write_layout_report};
use storage::{load_embeddings, write_layout};

type LayoutPoint = [f32; 2];

/// Build and persist a 2D similarity-map layout for the given model embeddings.
///
/// The projection is currently computed with t-SNE and then written into the
/// existing `layout_umap` table so persisted data and older callers remain
/// compatible.
pub fn build_map_layout(
    conn: &mut Connection,
    model_id: &str,
    layout_version: &str,
    seed: u64,
    min_coverage: f32,
) -> Result<MapLayoutReport, String> {
    let (sample_ids, vectors, dim) = load_embeddings(conn, model_id)?;
    let layout = build_layout(vectors, dim, seed)?;
    persist_and_validate_layout(conn, &sample_ids, &layout, model_id, layout_version)?;
    validate_layout(&layout, min_coverage)
}

/// Build and persist a 2D layout through the legacy UMAP-named entrypoint.
///
/// This forwards to [`build_map_layout`] while keeping the established API
/// shape used by older internal callers.
pub fn build_umap_layout(
    conn: &mut Connection,
    model_id: &str,
    umap_version: &str,
    seed: u64,
    min_coverage: f32,
) -> Result<UmapReport, String> {
    build_map_layout(conn, model_id, umap_version, seed, min_coverage)
}

/// Return the default JSON report path through the legacy UMAP-named helper.
pub fn default_report_path(db_path: &Path, umap_version: &str) -> std::path::PathBuf {
    default_layout_report_path(db_path, umap_version)
}

/// Serialize and write a layout report through the legacy UMAP-named helper.
pub fn write_report(path: &Path, report: &UmapReport) -> Result<(), String> {
    write_layout_report(path, report)
}

fn build_layout(vectors: Vec<f64>, dim: usize, seed: u64) -> Result<Vec<LayoutPoint>, String> {
    compute_tsne(vectors, dim, seed)
}

fn persist_and_validate_layout(
    conn: &mut Connection,
    sample_ids: &[String],
    layout: &[LayoutPoint],
    model_id: &str,
    layout_version: &str,
) -> Result<(), String> {
    let inserted = write_layout(conn, sample_ids, layout, model_id, layout_version)?;
    validate_insert_count(inserted, sample_ids.len())?;
    Ok(())
}

fn validate_insert_count(inserted: usize, expected: usize) -> Result<(), String> {
    if inserted != expected {
        return Err("Similarity map layout insert count mismatch".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_layout_tables(conn: &Connection) {
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
             CREATE TABLE layout_umap (
                sample_id TEXT PRIMARY KEY,
                model_id TEXT NOT NULL,
                umap_version TEXT NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                created_at INTEGER NOT NULL
            ) WITHOUT ROWID;",
        )
        .unwrap();
    }

    #[test]
    fn build_map_layout_rejects_missing_embeddings() {
        let mut conn = Connection::open_in_memory().unwrap();
        create_layout_tables(&conn);

        let err = build_map_layout(&mut conn, "missing-model", "v1", 0, 0.5).unwrap_err();
        assert!(err.contains("No embeddings found"));
    }

    #[test]
    fn legacy_umap_helpers_delegate_to_layout_helpers() {
        let path = Path::new("/tmp/library/source.db");
        assert_eq!(
            default_report_path(path, "v2"),
            default_layout_report_path(path, "v2")
        );

        let dir = tempdir().unwrap();
        let report_path = dir.path().join("report.json");
        let report = MapLayoutReport {
            total: 4,
            valid: 4,
            invalid: 0,
            coverage_ratio: 1.0,
            x_min: -1.0,
            x_max: 1.0,
            y_min: -2.0,
            y_max: 2.0,
        };
        write_report(&report_path, &report).expect("write legacy report");
        let written = std::fs::read_to_string(&report_path).expect("read legacy report");
        assert!(written.contains("\"coverage_ratio\": 1.0"));

        let mut conn = Connection::open_in_memory().unwrap();
        create_layout_tables(&conn);
        let legacy_err = build_umap_layout(&mut conn, "missing-model", "v1", 0, 0.5).unwrap_err();
        let map_err = build_map_layout(&mut conn, "missing-model", "v1", 0, 0.5).unwrap_err();
        assert_eq!(legacy_err, map_err);
    }
}
