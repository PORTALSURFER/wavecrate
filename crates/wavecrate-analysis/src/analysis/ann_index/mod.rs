#[cfg(test)]
pub(crate) mod build;
#[cfg(not(test))]
mod build;
mod container;
mod publication;
#[cfg(test)]
pub(crate) mod state;
#[cfg(not(test))]
mod state;
#[cfg(test)]
pub(crate) mod storage;
#[cfg(not(test))]
mod storage;

use crate::analysis::{decode_f32_le_blob, similarity};
use rusqlite::{Connection, TransactionBehavior};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

/// Neighbor result returned by ANN similarity search.
#[derive(Debug)]
pub struct SimilarNeighbor {
    /// Sample identifier for the neighbor.
    pub sample_id: String,
    /// Distance between the query and the neighbor (lower is more similar).
    pub distance: f32,
}

static ANN_INDEX: LazyLock<RwLock<HashMap<String, Arc<RwLock<state::AnnIndexState>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Exact version fingerprint for the ANN algorithm, parameters, and container schema.
pub fn contract_version() -> String {
    let params = state::default_params();
    let payload = format!(
        "ann_container_v{}:{}",
        container::ANN_CONTAINER_VERSION,
        serde_json::to_string(&params).expect("ANN parameters must serialize")
    );
    format!("ann_v1_{}", blake3::hash(payload.as_bytes()).to_hex())
}

/// Get the index state wrapper, loading it if necessary.
/// This minimizes the time the global lock is held.
fn get_index_entry(conn: &Connection) -> Result<Arc<RwLock<state::AnnIndexState>>, String> {
    let key = storage::index_key(conn)?;

    // Fast path: check with read lock
    {
        let guard = ANN_INDEX.read().map_err(|_| "ANN index lock poisoned")?;
        if let Some(state) = guard.get(&key) {
            return Ok(state.clone());
        }
    }

    // Slow path: load the published current index without holding global lock
    // Note: This might do redundant work if multiple threads race here,
    // but better than blocking the world.
    let loaded_state = build::load_current_index(conn)?;
    let loaded_state = Arc::new(RwLock::new(loaded_state));

    // Write lock to insert
    let mut guard = ANN_INDEX.write().map_err(|_| "ANN index lock poisoned")?;
    // Double-check in case another thread won the race
    if let Some(state) = guard.get(&key) {
        return Ok(state.clone());
    }

    guard.insert(key, loaded_state.clone());
    Ok(loaded_state)
}

fn with_index_state_read<R>(
    conn: &Connection,
    f: impl FnOnce(&state::AnnIndexState) -> Result<R, String>,
) -> Result<R, String> {
    let state_arc = get_index_entry(conn)?;
    let guard = state_arc
        .read()
        .map_err(|_| "ANN index state lock poisoned")?;
    f(&guard)
}

/// Find the `k` nearest neighbors for a stored sample id.
pub fn find_similar(
    conn: &Connection,
    sample_id: &str,
    k: usize,
) -> Result<Vec<SimilarNeighbor>, String> {
    if k == 0 {
        return Ok(Vec::new());
    }
    let embedding = load_embedding(conn, sample_id)?;

    with_index_state_read(conn, |state| {
        if !state.id_lookup.contains_key(sample_id) {
            return Err(format!(
                "Sample {sample_id} is not present in the current ANN generation"
            ));
        }
        perform_search(state, &embedding, k, Some(sample_id))
    })
}

/// Find the `k` nearest neighbors for an in-memory embedding.
pub fn find_similar_for_embedding(
    conn: &Connection,
    embedding: &[f32],
    k: usize,
) -> Result<Vec<SimilarNeighbor>, String> {
    if k == 0 {
        return Ok(Vec::new());
    }
    if embedding.len() != similarity::SIMILARITY_DIM {
        return Err(format!(
            "Embedding dim mismatch: expected {}, got {}",
            similarity::SIMILARITY_DIM,
            embedding.len()
        ));
    }
    with_index_state_read(conn, |state| {
        if state.id_map.is_empty() {
            return Err("ANN index has no embeddings".to_string());
        }
        perform_search(state, embedding, k, None)
    })
}

fn perform_search(
    state: &state::AnnIndexState,
    embedding: &[f32],
    k: usize,
    skip_id: Option<&str>,
) -> Result<Vec<SimilarNeighbor>, String> {
    let ef = state.params.ef_search.max(k + 1);
    let total = state.id_map.len();
    let mut requested = k + if skip_id.is_some() { 1 } else { 0 };
    if requested > total {
        requested = total;
    }
    let neighbours = state.hnsw.search(embedding, requested, ef);
    let mut results = Vec::with_capacity(neighbours.len());
    for neighbour in neighbours {
        if let Some(candidate) = state.id_map.get(neighbour.d_id) {
            if let Some(skip) = skip_id
                && candidate == skip
            {
                continue;
            }
            results.push(SimilarNeighbor {
                sample_id: candidate.clone(),
                distance: neighbour.distance,
            });
        }
    }
    results.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
    Ok(results)
}

pub(crate) fn publish_exact_index_with_transaction(
    conn: &mut Connection,
    embeddings: &[(String, Vec<f32>)],
    artifact_generation: &str,
    publication_fence: &impl Fn(&Connection) -> Result<bool, String>,
    publish_sql_artifacts: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<(), String>,
) -> Result<bool, String> {
    let params = state::default_params();
    let index_path = storage::generation_index_path(conn, artifact_generation)?;
    let mut state = Some(build::build_index_from_embeddings(
        embeddings, params, index_path,
    )?);
    let key = storage::index_key(conn)?;
    let mut publish_sql_artifacts = Some(publish_sql_artifacts);
    let existing = ANN_INDEX
        .read()
        .map_err(|_| "ANN index lock poisoned")?
        .get(&key)
        .cloned();
    if let Some(existing) = existing {
        return publish_exact_into_existing_state(
            conn,
            &mut state,
            existing,
            publication_fence,
            &mut publish_sql_artifacts,
        );
    }
    let mut registry = ANN_INDEX
        .write()
        .map_err(|_| "ANN index lock poisoned".to_string())?;
    if let Some(existing) = registry.get(&key).cloned() {
        drop(registry);
        return publish_exact_into_existing_state(
            conn,
            &mut state,
            existing,
            publication_fence,
            &mut publish_sql_artifacts,
        );
    }
    // Keep first-load publication private while the registry write lock prevents a
    // concurrent cache miss from installing database-derived state. A rejected or
    // errored fence therefore leaves no unpublished ANN state globally visible.
    if !publish_exact_transaction(
        conn,
        state.as_mut().expect("exact ANN state must be available"),
        publication_fence,
        &mut publish_sql_artifacts,
    )? {
        return Ok(false);
    }
    let state = state.take().expect("published ANN state must be available");
    registry.insert(key, Arc::new(RwLock::new(state)));
    drop(registry);
    storage::remove_retired_artifacts(conn);
    Ok(true)
}

fn publish_exact_into_existing_state<F>(
    conn: &mut Connection,
    state: &mut Option<state::AnnIndexState>,
    existing: Arc<RwLock<state::AnnIndexState>>,
    publication_fence: &impl Fn(&Connection) -> Result<bool, String>,
    publish_sql_artifacts: &mut Option<F>,
) -> Result<bool, String>
where
    F: FnOnce(&rusqlite::Transaction<'_>) -> Result<(), String>,
{
    let mut published_state = existing
        .write()
        .map_err(|_| "ANN index state lock poisoned")?;
    let previous_index_path = published_state.index_path.clone();
    if !publish_exact_transaction(
        conn,
        state.as_mut().expect("exact ANN state must be available"),
        publication_fence,
        publish_sql_artifacts,
    )? {
        return Ok(false);
    }
    let state = state.take().expect("published ANN state must be available");
    let published_index_path = state.index_path.clone();
    *published_state = state;
    drop(published_state);
    storage::remove_superseded_generation(&previous_index_path, &published_index_path);
    storage::remove_retired_artifacts(conn);
    Ok(true)
}

fn publish_exact_transaction<F>(
    conn: &mut Connection,
    state: &mut state::AnnIndexState,
    publication_fence: &impl Fn(&Connection) -> Result<bool, String>,
    publish_sql_artifacts: &mut Option<F>,
) -> Result<bool, String>
where
    F: FnOnce(&rusqlite::Transaction<'_>) -> Result<(), String>,
{
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Start exact ANN publication transaction failed: {err}"))?;
    if !publication_fence(&tx)? {
        tx.rollback()
            .map_err(|err| format!("Roll back stale exact ANN publication failed: {err}"))?;
        return Ok(false);
    }
    publish_sql_artifacts
        .take()
        .ok_or_else(|| "Exact similarity SQL publication was already consumed".to_string())?(
        &tx
    )?;
    publication::publish_index(&tx, state)?;
    tx.execute(
        "DELETE FROM metadata
         WHERE key IN ('ann_index_dirty_v1', 'last_similarity_prep_scan_at')",
        [],
    )
    .map_err(|err| format!("Remove retired similarity metadata failed: {err}"))?;
    tx.commit()
        .map_err(|err| format!("Commit exact similarity publication failed: {err}"))?;
    Ok(true)
}

#[cfg(test)]
pub(crate) fn evict_index_for_test(conn: &Connection) -> Result<(), String> {
    let key = storage::index_key(conn)?;
    ANN_INDEX
        .write()
        .map_err(|_| "ANN index lock poisoned".to_string())?
        .remove(&key);
    Ok(())
}

#[cfg(test)]
pub(crate) fn index_is_cached_for_test(conn: &Connection) -> Result<bool, String> {
    let key = storage::index_key(conn)?;
    Ok(ANN_INDEX
        .read()
        .map_err(|_| "ANN index lock poisoned".to_string())?
        .contains_key(&key))
}

fn load_embedding(conn: &Connection, sample_id: &str) -> Result<Vec<f32>, String> {
    let blob: Vec<u8> = conn
        .query_row(
            "SELECT vec FROM embeddings WHERE sample_id = ?1 AND model_id = ?2",
            rusqlite::params![sample_id, similarity::SIMILARITY_MODEL_ID],
            |row| row.get(0),
        )
        .map_err(|err| format!("Failed to load embedding for {sample_id}: {err}"))?;
    decode_f32_le_blob(&blob)
}
