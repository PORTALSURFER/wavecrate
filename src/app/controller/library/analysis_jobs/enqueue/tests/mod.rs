use super::enqueue_samples::enqueue_jobs_for_source;
use crate::app::controller::library::analysis_jobs::db;
use crate::app_dirs::ConfigBaseGuard;
use crate::sample_sources::scanner::ChangedSample;
use crate::sample_sources::{SampleSource, SourceDatabase};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::{TempDir, tempdir};

mod invalidation;

struct TestEnv {
    _config_dir: TempDir,
    _config_guard: ConfigBaseGuard,
    _source_dir: TempDir,
    source: SampleSource,
}

impl TestEnv {
    fn new() -> Self {
        let config_dir = tempdir().unwrap();
        let config_guard = ConfigBaseGuard::set(config_dir.path().to_path_buf());
        let source_dir = tempdir().unwrap();
        let source = SampleSource::new(source_dir.path().to_path_buf());
        crate::sample_sources::library::save(&crate::sample_sources::library::LibraryState {
            sources: vec![source.clone()],
        })
        .unwrap();
        Self {
            _config_dir: config_dir,
            _config_guard: config_guard,
            _source_dir: source_dir,
            source,
        }
    }

    fn create_files(&self, files: &[&str]) {
        for file in files {
            let path = self.source.root.join(file);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, b"test").unwrap();
        }
    }
}

fn seed_source_db(source: &SampleSource, entries: &[(&str, &str)]) {
    let source_db = SourceDatabase::open(&source.root).unwrap();
    let mut batch = source_db.write_batch().unwrap();
    for (path, hash) in entries {
        batch
            .upsert_file_with_hash(Path::new(path), 1, 1, hash)
            .unwrap();
    }
    batch.commit().unwrap();
}

fn clear_analysis_tables(conn: &Connection) {
    conn.execute_batch(
        "DELETE FROM analysis_jobs;
         DELETE FROM samples;
         DELETE FROM features;
         DELETE FROM embeddings;
         DELETE FROM similarity_aspect_descriptors;
         DELETE FROM analysis_cache_aspect_descriptors;",
    )
    .unwrap();
}

fn sample_id(source: &SampleSource, relative_path: &str) -> String {
    format!("{}::{}", source.id.as_str(), relative_path)
}

fn insert_sample_row(conn: &Connection, sample_id: &str, hash: &str, version: Option<&str>) {
    if let Some(version) = version {
        conn.execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version)
             VALUES (?1, ?2, 1, 1, NULL, NULL, ?3)",
            params![sample_id, hash, version],
        )
        .unwrap();
    } else {
        conn.execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version)
             VALUES (?1, ?2, 1, 1, NULL, NULL, NULL)",
            params![sample_id, hash],
        )
        .unwrap();
    }
}

fn insert_features_row(conn: &Connection, sample_id: &str) {
    conn.execute(
        "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
         VALUES (?1, 1, X'01020304', 1)",
        params![sample_id],
    )
    .unwrap();
}

fn insert_embeddings_row(conn: &Connection, sample_id: &str, model_id: &str) {
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, 1, X'01020304', 0)",
        params![
            sample_id,
            model_id,
            wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32
        ],
    )
    .unwrap();
}

fn insert_aspect_descriptors_row(conn: &Connection, sample_id: &str) {
    conn.execute(
        "INSERT INTO similarity_aspect_descriptors
         (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, 0)",
        params![
            sample_id,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            wavecrate_analysis::aspects::all_aspect_mask() as i64,
            vec![0_u8; wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4]
        ],
    )
    .unwrap();
}
