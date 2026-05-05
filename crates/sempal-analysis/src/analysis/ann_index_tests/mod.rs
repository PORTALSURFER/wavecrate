use crate::analysis::ann_index::{
    build as ann_build, storage as ann_storage, update as ann_update,
};
use crate::analysis::vector::encode_f32_le_blob;
use crate::analysis::{ann_index, similarity};
use crate::app_dirs::ConfigBaseGuard;
use hnsw_rs::api::AnnT;
use rusqlite::{Connection, params};
use std::sync::{LazyLock, Mutex};
use tempfile::tempdir;

pub(super) static ANN_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

mod query;
mod storage;
mod updates;

pub(super) fn unit_vec(dim: usize, idx: usize) -> Vec<f32> {
    let mut vec = vec![0.0; dim];
    if idx < dim {
        vec[idx] = 1.0;
    }
    vec
}

pub(super) fn blend_unit(dim: usize, a: usize, b: usize, mix: f32) -> Vec<f32> {
    let mut vec = unit_vec(dim, a);
    if b < dim {
        vec[b] = mix;
    }
    vec
}

pub(super) fn normalize(mut vec: Vec<f32>) -> Vec<f32> {
    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vec {
            *value /= norm;
        }
    }
    vec
}

pub(super) fn insert_embeddings(conn: &Connection, dim: usize, samples: &[(&str, Vec<f32>)]) {
    for (sample_id, vec) in samples {
        let blob = encode_f32_le_blob(vec);
        conn.execute(
            "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
            params![sample_id, similarity::SIMILARITY_MODEL_ID, dim as i64, blob],
        )
        .unwrap();
    }
}

fn create_ann_tables(conn: &Connection) {
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
         CREATE TABLE ann_index_meta (
            model_id TEXT PRIMARY KEY,
            index_path TEXT NOT NULL,
            count INTEGER NOT NULL,
            params_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        ) WITHOUT ROWID;",
    )
    .unwrap();
}

pub(super) fn write_legacy_ann_files(
    state: &crate::analysis::ann_index::state::AnnIndexState,
    path: &std::path::Path,
) {
    let dir = path.parent().expect("legacy parent");
    std::fs::create_dir_all(dir).unwrap();
    let basename = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("legacy name");
    state.hnsw.file_dump(dir, basename).unwrap();
    let id_map_path = ann_storage::legacy_id_map_path_for(path);
    ann_storage::save_legacy_id_map(&id_map_path, &state.id_map).unwrap();
}

pub(super) fn with_ann_test_db<T>(f: impl FnOnce(&Connection) -> T) -> T {
    let _lock = ANN_TEST_LOCK.lock().expect("ann test lock poisoned");
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let conn = Connection::open_in_memory().unwrap();
    create_ann_tables(&conn);
    f(&conn)
}

pub(super) fn load_disk_state(
    conn: &Connection,
) -> crate::analysis::ann_index::state::AnnIndexState {
    let meta = ann_storage::read_meta(conn, similarity::SIMILARITY_MODEL_ID)
        .unwrap()
        .expect("ann meta");
    ann_build::load_index_from_disk(conn, &meta)
        .unwrap()
        .expect("ann load")
        .state
}

pub(super) fn basic_samples(dim: usize) -> Vec<(&'static str, Vec<f32>)> {
    vec![
        ("s1", normalize(unit_vec(dim, 0))),
        ("s2", normalize(unit_vec(dim, 1))),
        ("s3", normalize(unit_vec(dim, 2))),
    ]
}

pub(super) fn brute_force_neighbors<'a>(
    target: &str,
    samples: &'a [(&'a str, Vec<f32>)],
    k: usize,
) -> Vec<&'a str> {
    let target_vec = samples
        .iter()
        .find(|(id, _)| *id == target)
        .expect("target sample")
        .1
        .as_slice();
    let mut scored: Vec<(&'a str, f32)> = samples
        .iter()
        .filter(|(id, _)| *id != target)
        .map(|(id, vec)| (*id, cosine_distance(target_vec, vec)))
        .collect();
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    scored.into_iter().take(k).map(|(id, _)| id).collect()
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let mut dot = 0.0;
    for i in 0..len {
        dot += a[i] * b[i];
    }
    1.0 - dot
}

pub(super) fn assert_results_within_top_k(
    target: &str,
    samples: &[(&str, Vec<f32>)],
    k: usize,
    result_ids: &[&str],
) {
    let target_vec = samples
        .iter()
        .find(|(id, _)| *id == target)
        .expect("target sample")
        .1
        .as_slice();
    let mut scored: Vec<(&str, f32)> = samples
        .iter()
        .filter(|(id, _)| *id != target)
        .map(|(id, vec)| (*id, cosine_distance(target_vec, vec)))
        .collect();
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let threshold = scored
        .get(k.saturating_sub(1))
        .map(|entry| entry.1)
        .unwrap_or(f32::INFINITY);
    for id in result_ids {
        let distance = scored
            .iter()
            .find(|(entry_id, _)| entry_id == id)
            .map(|entry| entry.1)
            .expect("result id present");
        assert!(
            distance <= threshold + 1e-6,
            "result {id} distance {distance} exceeds threshold {threshold}"
        );
    }
}
