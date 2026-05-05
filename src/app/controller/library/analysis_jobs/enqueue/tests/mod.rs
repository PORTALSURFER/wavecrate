use super::super::wakeup;
use super::enqueue_embeddings::enqueue_jobs_for_embedding_backfill;
use super::enqueue_samples::{
    enqueue_jobs_for_source, enqueue_jobs_for_source_backfill,
    enqueue_jobs_for_source_backfill_full, enqueue_jobs_for_source_missing_features,
};
use crate::app::controller::library::analysis_jobs::db;
use crate::app_dirs::ConfigBaseGuard;
use crate::sample_sources::scanner::ChangedSample;
use crate::sample_sources::{SampleSource, SourceDatabase};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tempfile::{TempDir, tempdir};

mod backfill;
mod invalidation;

struct TestEnv {
    _config_dir: TempDir,
    _config_guard: ConfigBaseGuard,
    _source_dir: TempDir,
    source: SampleSource,
}

static CLAIM_WAKEUP_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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
         DELETE FROM embeddings;",
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
            crate::analysis::similarity::SIMILARITY_DIM as i64,
            crate::analysis::similarity::SIMILARITY_DTYPE_F32
        ],
    )
    .unwrap();
}
