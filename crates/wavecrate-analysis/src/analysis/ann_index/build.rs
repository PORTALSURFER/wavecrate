use super::container;
use super::state::{
    AnnHnsw, AnnIndexParams, AnnIndexState, LoadedAnnHnsw, build_id_lookup, default_params,
};
use super::storage::{generation_index_path, read_meta};
use hnsw_rs::prelude::*;
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tempfile::Builder;

const TEMP_UNPACK_BASENAME: &str = "ann_unpack";

#[derive(Deserialize)]
struct CurrentSimilarityArtifactState {
    state: String,
    artifact_generation: String,
    model_id: String,
}

/// Load the exact ANN generation named by the current database metadata.
pub(crate) fn load_current_index(conn: &Connection) -> Result<AnnIndexState, String> {
    let params = default_params();
    let artifact_state = read_current_artifact_state(conn)?.ok_or_else(|| {
        "Current similarity artifact generation has not been published".to_string()
    })?;
    if artifact_state.state != "current" || artifact_state.model_id != params.model_id {
        return Err("Current similarity artifact state uses a different contract".to_string());
    }
    let meta = read_meta(conn, &params.model_id)?
        .ok_or_else(|| "Current ANN generation has not been published".to_string())?;
    if meta.params != params {
        return Err("Current ANN generation uses a different contract version".to_string());
    }
    let expected_path = generation_index_path(conn, &artifact_state.artifact_generation)?;
    if meta.index_path != expected_path {
        return Err(
            "Current ANN metadata does not name the published artifact generation".to_string(),
        );
    }
    load_container_index(&meta.index_path, &meta.params)?
        .ok_or_else(|| "Current ANN generation container is missing or invalid".to_string())
}

fn read_current_artifact_state(
    conn: &Connection,
) -> Result<Option<CurrentSimilarityArtifactState>, String> {
    let state = conn
        .query_row(
            "SELECT value FROM metadata WHERE key = 'similarity_artifact_state_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("Failed to read current similarity artifact state: {error}"))?;
    state
        .map(|state| {
            serde_json::from_str(&state)
                .map_err(|error| format!("Invalid current similarity artifact state: {error}"))
        })
        .transpose()
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
