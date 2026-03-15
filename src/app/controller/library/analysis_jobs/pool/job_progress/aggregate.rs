use super::DB_REFRESH_INTERVAL;
use super::source_discovery::ProgressSourceDb;
use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use super::super::progress_cache::ProgressCache;

/// Aggregate progress across the currently opened source databases.
pub(super) fn current_progress_all(
    sources: &mut [ProgressSourceDb],
    progress_cache: &Arc<RwLock<ProgressCache>>,
    refresh_cache: bool,
) -> AnalysisProgress {
    if refresh_cache
        || progress_cache
            .read()
            .map(|cache| cache.is_empty())
            .unwrap_or(true)
    {
        let mut total = AnalysisProgress::default();
        let mut updates = Vec::new();
        for source in sources {
            if let Ok(progress) = db::current_progress(&source.conn) {
                total.pending += progress.pending;
                total.running += progress.running;
                total.done += progress.done;
                total.failed += progress.failed;
                total.samples_total += progress.samples_total;
                total.samples_pending_or_running += progress.samples_pending_or_running;
                updates.push((source.source_id.clone(), progress));
            }
        }
        if let Ok(mut cache) = progress_cache.write() {
            cache.update_many(updates);
        }
        return total;
    }
    if let Ok(cache) = progress_cache.read() {
        return cache.total_for_sources(sources.iter().map(|source| &source.source_id));
    }
    AnalysisProgress::default()
}

/// Decide whether the poller should refresh progress from the DB instead of
/// reusing the cached aggregate.
pub(super) fn should_refresh_db(
    last_db_refresh: Instant,
    progress_cache: &Arc<RwLock<ProgressCache>>,
) -> bool {
    if last_db_refresh.elapsed() >= DB_REFRESH_INTERVAL {
        return true;
    }
    progress_cache
        .read()
        .map(|cache| cache.is_empty())
        .unwrap_or(true)
}
