use super::source_discovery::ProgressSourceDb;
use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;
use std::sync::{Arc, RwLock};

use super::super::progress_cache::ProgressCache;

/// Aggregate progress across the currently tracked source ids.
pub(super) fn current_progress_all(
    sources: &[ProgressSourceDb],
    progress_cache: &Arc<RwLock<ProgressCache>>,
) -> AnalysisProgress {
    if let Ok(cache) = progress_cache.read() {
        return cache.total_for_sources(sources.iter().map(|source| &source.source_id));
    }
    AnalysisProgress::default()
}

/// Seed missing cache entries from the current source DBs.
pub(super) fn seed_missing_progress(
    sources: &[ProgressSourceDb],
    progress_cache: &Arc<RwLock<ProgressCache>>,
) -> bool {
    let missing = progress_cache
        .read()
        .map(|cache| {
            sources
                .iter()
                .filter(|source| !cache.contains(&source.source_id))
                .map(|source| source.source_id.clone())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    if missing.is_empty() {
        return false;
    }
    let mut updates = Vec::new();
    for source in sources {
        if !missing.contains(&source.source_id) {
            continue;
        }
        match db::current_progress(&source.conn, &source.source_root) {
            Ok(progress) => updates.push((source.source_id.clone(), progress)),
            Err(err) => {
                tracing::warn!(
                    source_id = %source.source_id,
                    source_root = %source.source_root.display(),
                    error = %err,
                    "Failed to seed analysis progress cache for source"
                );
            }
        }
    }
    if updates.is_empty() {
        return false;
    }
    if let Ok(mut cache) = progress_cache.write() {
        cache.update_many(updates);
        return true;
    }
    false
}
