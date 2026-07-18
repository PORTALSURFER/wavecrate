use super::state::{AnnIndexMetaRow, AnnIndexState};
use crate::app_dirs;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};

const ANN_CONTAINER_NAME: &str = "similarity_hnsw.ann";
const RETIRED_ANN_DIR: &str = "ann";
const RETIRED_ANN_FILES: &[&str] = &[
    "similarity_hnsw.hnsw.graph",
    "similarity_hnsw.hnsw.data",
    "similarity_hnsw.idmap.json",
];

/// Load ANN metadata for the given model id, if present.
pub(crate) fn read_meta(
    conn: &Connection,
    model_id: &str,
) -> Result<Option<AnnIndexMetaRow>, String> {
    let row = conn
        .query_row(
            "SELECT index_path, params_json FROM ann_index_meta WHERE model_id = ?1",
            params![model_id],
            |row| {
                let path: String = row.get(0)?;
                let params_json: String = row.get(1)?;
                Ok((path, params_json))
            },
        )
        .optional()
        .map_err(|err| format!("Failed to read ann_index_meta: {err}"))?;
    let Some((path, params_json)) = row else {
        return Ok(None);
    };
    let params: super::state::AnnIndexParams =
        serde_json::from_str(&params_json).map_err(|err| format!("{err}"))?;
    let index_path = PathBuf::from(path);
    Ok(Some(AnnIndexMetaRow { index_path, params }))
}

/// Insert or update ANN metadata for the current state.
pub(crate) fn upsert_meta(conn: &Connection, state: &AnnIndexState) -> Result<(), String> {
    let params_json = serde_json::to_string(&state.params).map_err(|err| format!("{err}"))?;
    let now = chrono_now_epoch_seconds();
    conn.execute(
        "INSERT INTO ann_index_meta (model_id, index_path, count, params_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(model_id) DO UPDATE SET
           index_path = excluded.index_path,
           count = excluded.count,
           params_json = excluded.params_json,
           updated_at = excluded.updated_at",
        params![
            state.params.model_id.as_str(),
            state.index_path.to_string_lossy(),
            state.id_map.len() as i64,
            params_json,
            now
        ],
    )
    .map_err(|err| format!("Failed to update ann_index_meta: {err}"))?;
    Ok(())
}

/// Produce a stable cache key for ANN state keyed by the source database.
pub(crate) fn index_key(conn: &Connection) -> Result<String, String> {
    let index_path = default_index_path(conn)?;
    Ok(index_path.to_string_lossy().to_string())
}

/// Resolve the current ANN container path for a source database.
pub(crate) fn default_index_path(conn: &Connection) -> Result<PathBuf, String> {
    let root = match database_root_dir(conn) {
        Ok(dir) => dir,
        Err(_) => app_dirs::app_root_dir().map_err(|err| err.to_string())?,
    };
    std::fs::create_dir_all(&root).map_err(|err| format!("Failed to create ANN dir: {err}"))?;
    Ok(root.join(ANN_CONTAINER_NAME))
}

/// Best-effort removal for an obsolete generation container in the same source database.
pub(crate) fn remove_superseded_generation(previous: &Path, current: &Path) {
    if previous == current || previous.parent() != current.parent() {
        return;
    }
    let Some(name) = previous.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if name.starts_with("similarity_hnsw.") && name.ends_with(".ann") {
        let _ = std::fs::remove_file(previous);
    }
}

/// Best-effort removal of files owned exclusively by the retired split-file ANN format.
pub(crate) fn remove_retired_artifacts(conn: &Connection) {
    let Ok(root) = database_root_dir(conn) else {
        return;
    };
    let retired_dir = root.join(RETIRED_ANN_DIR);
    for filename in RETIRED_ANN_FILES {
        let path = retired_dir.join(filename);
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
    let _ = std::fs::remove_dir(retired_dir);
}

/// Return the directory that contains the source database file.
pub(crate) fn database_root_dir(conn: &Connection) -> Result<PathBuf, String> {
    let mut stmt = conn
        .prepare("PRAGMA database_list")
        .map_err(|err| format!("Failed to read database_list: {err}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|err| format!("Failed to read database_list: {err}"))?;
    let Some(row) = rows
        .next()
        .map_err(|err| format!("Failed to read database_list: {err}"))?
    else {
        return Err("Missing database_list row".to_string());
    };
    let path: Option<String> = row.get(2).map_err(|err| err.to_string())?;
    let path = path.filter(|value| !value.is_empty());
    let path = path.ok_or_else(|| "Database path missing".to_string())?;
    let path = PathBuf::from(path);
    let root = path
        .parent()
        .ok_or_else(|| "Database path missing parent".to_string())?;
    Ok(root.to_path_buf())
}

/// Return the graph/data paths for a given HNSW dump basename.
pub(crate) fn hnsw_dump_paths(index_path: &Path) -> Result<(PathBuf, PathBuf), String> {
    let basename = index_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Index path missing basename".to_string())?;
    let dir = index_path
        .parent()
        .ok_or_else(|| "Index path missing parent".to_string())?;
    let graph = dir.join(format!("{basename}.hnsw.graph"));
    let data = dir.join(format!("{basename}.hnsw.data"));
    Ok((graph, data))
}

fn chrono_now_epoch_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
