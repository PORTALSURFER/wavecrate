//! Retained filter-stage storage lookup, promotion, and insertion.

use super::super::super::*;
use super::WorkerFilteredStage;
use std::sync::Arc;

pub(super) fn reuse_cached_stage(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    entries_len: usize,
    filter_hash: u64,
) -> Option<WorkerFilteredStage> {
    let index = cache.filter_stage_cache.iter().position(|cached| {
        cached.source_id == source_id
            && cached.revision == cache.revision
            && cached.filter_hash == filter_hash
            && cached.accepts.len() == entries_len
    })?;
    let cached = cache.filter_stage_cache.remove(index);
    cache.filter_stage_cache.insert(0, cached);
    current_stage(cache)
}

pub(super) fn store_filter_stage(
    cache: &mut SearchWorkerCache,
    source_id: &str,
    filter_hash: u64,
    accepts: Vec<bool>,
    rows: Vec<usize>,
) -> Option<WorkerFilteredStage> {
    cache.filter_stage_cache.insert(
        0,
        WorkerFilterStageCacheEntry {
            source_id: source_id.to_string(),
            revision: cache.revision,
            filter_hash,
            accepts: Arc::from(accepts),
            rows: Arc::from(rows),
        },
    );
    cache
        .filter_stage_cache
        .truncate(cache.max_cached_filter_stages);
    current_stage(cache)
}

fn current_stage(cache: &SearchWorkerCache) -> Option<WorkerFilteredStage> {
    cache
        .filter_stage_cache
        .first()
        .map(|cached| WorkerFilteredStage {
            accepts: Arc::clone(&cached.accepts),
            rows: Arc::clone(&cached.rows),
        })
}
