use super::source_discovery::ProgressSourceDb;
use crate::app::controller::jobs::{JobMessage, JobMessageSender};
use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::types::AnalysisJobMessage;
use crate::gui::repaint::SharedRepaintSignal;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::super::progress_cache::ProgressCache;

/// Fail stale running jobs, refresh the touched-source cache entries, and emit
/// a progress update when the aggregate changes.
pub(super) fn cleanup_stale_jobs(
    sources: &mut [ProgressSourceDb],
    stale_before: i64,
    progress_cache: &Arc<RwLock<ProgressCache>>,
    tx: &JobMessageSender,
    signal: &Arc<SharedRepaintSignal>,
) -> usize {
    let mut changed = 0;
    let mut touched_sources = std::collections::HashSet::new();
    for source in &mut *sources {
        if let Ok((updated, source_ids)) =
            db::fail_stale_running_jobs_with_sources(&source.conn, stale_before)
            && updated > 0
        {
            changed += updated;
            for source_id in source_ids {
                touched_sources.insert(source_id);
            }
        }
    }
    if touched_sources.is_empty() {
        return changed;
    }
    let mut updates = Vec::new();
    for source in &mut *sources {
        if !touched_sources.contains(&source.source_id) {
            continue;
        }
        if let Ok(progress) = db::current_progress(&source.conn) {
            updates.push((source.source_id.clone(), progress));
        }
    }
    if let Ok(mut cache) = progress_cache.write() {
        cache.update_many(updates);
    }
    if let Ok(cache) = progress_cache.read() {
        let total = cache.total_for_sources(sources.iter().map(|source| &source.source_id));
        let _ = tx.send(JobMessage::Analysis(AnalysisJobMessage::Progress {
            source_id: None,
            progress: total,
        }));
        signal.request_repaint();
    }
    changed
}

/// Return the current Unix epoch in whole seconds for stale-job comparisons.
pub(super) fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}
