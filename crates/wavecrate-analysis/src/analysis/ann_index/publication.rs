use super::container;
use super::state::AnnIndexState;
use super::storage::{hnsw_dump_paths, upsert_meta};
use rusqlite::Connection;
use tempfile::Builder;

const ANN_TEMP_DUMP_BASENAME: &str = "ann_dump";

/// Publish the ANN index as the current single-file generation.
pub(crate) fn publish_index(conn: &Connection, state: &mut AnnIndexState) -> Result<(), String> {
    if state.id_map.is_empty() {
        upsert_meta(conn, state)?;
        return Ok(());
    }
    let index_path = state.index_path.clone();
    let dir = index_path
        .parent()
        .ok_or_else(|| "Index path missing parent".to_string())?;
    std::fs::create_dir_all(dir).map_err(|err| format!("Failed to create ANN dir: {err}"))?;
    let temp_dir = Builder::new()
        .prefix("ann_dump")
        .tempdir_in(dir)
        .map_err(|err| format!("Failed to create ANN dump dir: {err}"))?;
    dump_hnsw(state, temp_dir.path())?;
    let (graph_path, data_path) = hnsw_dump_paths(&temp_dir.path().join(ANN_TEMP_DUMP_BASENAME))?;
    container::write_container(
        &index_path,
        &state.params.model_id,
        &graph_path,
        &data_path,
        &state.id_map,
    )?;
    upsert_meta(conn, state)?;
    Ok(())
}

fn dump_hnsw(state: &AnnIndexState, dir: &std::path::Path) -> Result<(), String> {
    state
        .hnsw
        .file_dump(dir, ANN_TEMP_DUMP_BASENAME)
        .map_err(|err| format!("Failed to save ANN index: {err}"))?;
    Ok(())
}
