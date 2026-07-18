use super::container;
use super::state::{
    AnnHnsw, AnnIndexParams, AnnIndexState, LoadedAnnHnsw, build_id_lookup, default_params,
};
use super::storage::read_meta;
use hnsw_rs::prelude::*;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::Builder;

const TEMP_UNPACK_BASENAME: &str = "ann_unpack";

/// Load the exact ANN generation named by the current database metadata.
pub(crate) fn load_current_index(conn: &Connection) -> Result<AnnIndexState, String> {
    let params = default_params();
    let meta = read_meta(conn, &params.model_id)?
        .ok_or_else(|| "Current ANN generation has not been published".to_string())?;
    if meta.params != params {
        return Err("Current ANN generation uses a different contract version".to_string());
    }
    load_container_index(&meta.index_path, &meta.params)?
        .ok_or_else(|| "Current ANN generation container is missing or invalid".to_string())
}

pub(crate) fn build_index_from_embeddings(
    items: &[(String, Vec<f32>)],
    params: AnnIndexParams,
    index_path: PathBuf,
) -> Result<AnnIndexState, String> {
    let hnsw = build_hnsw(&params, items.len() as i64);
    let mut id_map = Vec::with_capacity(items.len());
    for (sample_id, embedding) in items {
        if embedding.len() != params.dim {
            return Err(format!(
                "Embedding dim mismatch: expected {}, got {} for {sample_id}",
                params.dim,
                embedding.len()
            ));
        }
        let id = id_map.len();
        id_map.push(sample_id.clone());
        hnsw.insert((embedding.as_slice(), id));
    }
    Ok(build_state(
        params,
        AnnHnsw::Built(hnsw),
        id_map,
        index_path,
    ))
}

fn build_hnsw(params: &AnnIndexParams, count: i64) -> Hnsw<'static, f32, DistCosine> {
    let max_elements = (count.max(1) as usize).max(1024);
    Hnsw::new(
        params.max_nb_connection,
        max_elements,
        params.max_layer,
        params.ef_construction,
        DistCosine {},
    )
}

fn build_state(
    params: AnnIndexParams,
    hnsw: AnnHnsw,
    id_map: Vec<String>,
    index_path: PathBuf,
) -> AnnIndexState {
    let id_lookup = build_id_lookup(&id_map);
    AnnIndexState {
        hnsw,
        id_map,
        id_lookup,
        params,
        index_path,
    }
}

fn load_container_index(
    index_path: &Path,
    params: &AnnIndexParams,
) -> Result<Option<AnnIndexState>, String> {
    if !index_path.is_file() {
        return Ok(None);
    }
    let temp_dir = temp_unpack_dir()?;
    let unpack =
        match container::unpack_container(index_path, temp_dir.path(), TEMP_UNPACK_BASENAME) {
            Ok(unpack) => unpack,
            Err(_) => return Ok(None),
        };
    if unpack.model_id != params.model_id {
        return Ok(None);
    }
    let hnsw = match load_hnsw(temp_dir.path(), TEMP_UNPACK_BASENAME) {
        Ok(hnsw) => hnsw,
        Err(_) => return Ok(None),
    };
    build_loaded_state(hnsw, unpack.id_map, params, index_path.to_path_buf())
}

fn load_hnsw(dir: &std::path::Path, basename: &str) -> Result<AnnHnsw, String> {
    LoadedAnnHnsw::load(dir, basename).map(AnnHnsw::Loaded)
}

fn build_loaded_state(
    hnsw: AnnHnsw,
    id_map: Vec<String>,
    params: &AnnIndexParams,
    index_path: PathBuf,
) -> Result<Option<AnnIndexState>, String> {
    if hnsw.get_nb_point() != id_map.len() {
        return Ok(None);
    }
    Ok(Some(build_state(params.clone(), hnsw, id_map, index_path)))
}

fn temp_unpack_dir() -> Result<tempfile::TempDir, String> {
    Builder::new()
        .prefix("ann_unpack")
        .tempdir()
        .map_err(|err| format!("Failed to create ANN temp dir: {err}"))
}
