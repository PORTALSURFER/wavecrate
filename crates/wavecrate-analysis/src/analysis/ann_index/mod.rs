#[cfg(test)]
pub(crate) mod build;
#[cfg(not(test))]
mod build;
mod container;
#[cfg(test)]
pub(crate) mod state;
#[cfg(not(test))]
mod state;
#[cfg(test)]
pub(crate) mod storage;
#[cfg(not(test))]
mod storage;
#[cfg(test)]
pub(crate) mod update;
#[cfg(not(test))]
mod update;

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

    // Slow path: load or build index without holding global lock
    // Note: This might do redundant work if multiple threads race here,
    // but better than blocking the world.
    let loaded_state = build::load_or_build_index(conn)?;
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

fn with_index_state_mut<R>(
    conn: &Connection,
    f: impl FnOnce(&mut state::AnnIndexState) -> Result<R, String>,
) -> Result<R, String> {
    let state_arc = get_index_entry(conn)?;
    let mut guard = state_arc
        .write()
        .map_err(|_| "ANN index state lock poisoned")?;
    f(&mut guard)
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

/// Insert a single embedding into the ANN index when the sample id is absent.
///
/// Existing sample ids are left unchanged. This API currently does not replace
/// or rebuild an already indexed vector in place; callers that need to refresh
/// the full index should use [`crate::analysis::rebuild_ann_index`].
pub fn upsert_embedding(
    conn: &Connection,
    sample_id: &str,
    embedding: &[f32],
) -> Result<(), String> {
    with_index_state_mut(conn, |state| {
        update::upsert_embedding(conn, state, sample_id, embedding)
    })
}

/// Insert a batch of embeddings into the ANN index, skipping existing ids.
///
/// This preserves the current in-memory index contents for duplicate sample
/// ids and only appends embeddings that are not already present.
pub fn upsert_embeddings_batch<'a, I>(conn: &Connection, items: I) -> Result<(), String>
where
    I: IntoIterator<Item = (&'a str, &'a [f32])>,
{
    let mut iter = items.into_iter().peekable();
    if iter.peek().is_none() {
        return Ok(());
    }
    with_index_state_mut(conn, |state| {
        update::upsert_embeddings_batch(conn, state, iter)
    })
}

/// Flush any buffered ANN insertions to the on-disk index.
pub fn flush_pending_inserts(conn: &Connection) -> Result<(), String> {
    with_index_state_mut(conn, |state| update::flush_pending_inserts(conn, state))
}

/// Flush buffered ANN insertions only while a transactional generation fence is current.
///
/// The immediate SQLite transaction remains open while the index container and its database
/// metadata are published, preventing a source mutation from crossing the generation check.
pub fn flush_pending_inserts_with_publication_fence(
    conn: &mut Connection,
    publication_fence: &impl Fn(&Connection) -> Result<bool, String>,
) -> Result<bool, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Start ANN publication transaction failed: {err}"))?;
    if !publication_fence(&tx)? {
        tx.rollback()
            .map_err(|err| format!("Roll back stale ANN publication failed: {err}"))?;
        return Ok(false);
    }
    let state_arc = get_index_entry(&tx)?;
    let mut state = state_arc
        .write()
        .map_err(|_| "ANN index state lock poisoned")?;
    update::flush_pending_inserts(&tx, &mut state)?;
    tx.commit()
        .map_err(|err| format!("Commit ANN publication failed: {err}"))?;
    Ok(true)
}

/// Find the `k` nearest neighbors for a stored sample id.
///
/// If the sample exists in the embeddings table but is missing from the
/// current ANN index cache, this call lazily backfills the missing ANN entry
/// before searching.
pub fn find_similar(
    conn: &Connection,
    sample_id: &str,
    k: usize,
) -> Result<Vec<SimilarNeighbor>, String> {
    if k == 0 {
        return Ok(Vec::new());
    }
    let embedding = load_embedding(conn, sample_id)?;

    // Optimistic read
    {
        let state_arc = get_index_entry(conn)?;
        let state = state_arc
            .read()
            .map_err(|_| "ANN index state lock poisoned")?;

        // If the ID is already known, we can search with just the read lock
        if state.id_lookup.contains_key(sample_id) {
            let results = perform_search(&state, &embedding, k, Some(sample_id))?;
            if results.len() >= k {
                return Ok(results);
            }
        }
        // If not found, drop read lock and fall through to write path
    }

    // Write path: update index with missing ID then search
    with_index_state_mut(conn, |state| {
        if !state.id_lookup.contains_key(sample_id) {
            update::upsert_embedding(conn, state, sample_id, embedding.as_slice())?;
        }
        let results = perform_search(state, &embedding, k, Some(sample_id))?;
        if results.len() >= k {
            return Ok(results);
        }
        fallback_neighbors(conn, &embedding, k, Some(sample_id))
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
        let results = perform_search(state, embedding, k, None)?;
        if results.len() >= k {
            return Ok(results);
        }
        fallback_neighbors(conn, embedding, k, None)
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

fn fallback_neighbors(
    conn: &Connection,
    embedding: &[f32],
    k: usize,
    skip_id: Option<&str>,
) -> Result<Vec<SimilarNeighbor>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT sample_id, vec
             FROM embeddings
             WHERE model_id = ?1",
        )
        .map_err(|err| format!("Failed to query embeddings: {err}"))?;
    let mut rows = stmt
        .query(rusqlite::params![similarity::SIMILARITY_MODEL_ID])
        .map_err(|err| format!("Failed to iterate embeddings: {err}"))?;
    let mut scored: Vec<SimilarNeighbor> = Vec::new();
    while let Some(row) = rows.next().map_err(|err| err.to_string())? {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        if skip_id == Some(sample_id.as_str()) {
            continue;
        }
        let blob: Vec<u8> = row.get(1).map_err(|err| err.to_string())?;
        let candidate = decode_f32_le_blob(&blob)?;
        if candidate.len() != embedding.len() {
            continue;
        }
        let distance = cosine_distance(embedding, &candidate);
        scored.push(SimilarNeighbor {
            sample_id,
            distance,
        });
    }
    scored.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(k);
    Ok(scored)
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let mut dot = 0.0;
    for i in 0..len {
        dot += a[i] * b[i];
    }
    1.0 - dot
}

/// Rebuild the ANN index from the embeddings stored in the database.
pub fn rebuild_index(conn: &Connection) -> Result<(), String> {
    let params = state::default_params();
    let index_path = storage::default_index_path(conn)?;
    let mut state = build::build_index_from_db(conn, params, index_path)?;
    update::flush_index(conn, &mut state)?;
    let key = storage::index_key(conn)?;

    let wrapped_state = Arc::new(RwLock::new(state));
    let mut guard = ANN_INDEX
        .write()
        .map_err(|_| "ANN index lock poisoned".to_string())?;
    guard.insert(key, wrapped_state);
    Ok(())
}

/// Rebuild and publish the complete ANN index only while an exact fence remains current.
pub fn rebuild_index_with_publication_fence(
    conn: &mut Connection,
    publication_fence: &impl Fn(&Connection) -> Result<bool, String>,
) -> Result<bool, String> {
    let params = state::default_params();
    let index_path = storage::default_index_path(conn)?;
    let mut state = build::build_index_from_db(conn, params, index_path)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Start ANN rebuild publication transaction failed: {err}"))?;
    if !publication_fence(&tx)? {
        tx.rollback()
            .map_err(|err| format!("Roll back stale ANN rebuild failed: {err}"))?;
        return Ok(false);
    }
    update::flush_index(&tx, &mut state)?;
    tx.commit()
        .map_err(|err| format!("Commit ANN rebuild publication failed: {err}"))?;
    let key = storage::index_key(conn)?;
    let wrapped_state = Arc::new(RwLock::new(state));
    let mut guard = ANN_INDEX
        .write()
        .map_err(|_| "ANN index lock poisoned".to_string())?;
    guard.insert(key, wrapped_state);
    Ok(true)
}

pub(crate) fn publish_exact_index_with_transaction(
    conn: &mut Connection,
    embeddings: &[(String, Vec<f32>)],
    artifact_generation: &str,
    publication_fence: &impl Fn(&Connection) -> Result<bool, String>,
    publish_sql_artifacts: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<(), String>,
) -> Result<bool, String> {
    let params = state::default_params();
    let default_path = storage::default_index_path(conn)?;
    let parent = default_path
        .parent()
        .ok_or_else(|| "ANN index path missing parent".to_string())?;
    let generation = artifact_generation
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(32)
        .collect::<String>();
    if generation.is_empty() {
        return Err("ANN artifact generation must not be empty".to_string());
    }
    let index_path = parent.join(format!("similarity_hnsw.{generation}.ann"));
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
    let published_index_path = state.index_path.clone();
    registry.insert(key, Arc::new(RwLock::new(state)));
    drop(registry);
    storage::remove_superseded_generation(&default_path, &published_index_path);
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
    update::flush_index(&tx, state)?;
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
