use crate::app::controller::library::analysis_jobs::db;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use super::SOURCE_REFRESH_INTERVAL;

/// Open source database handle tracked by the progress poller.
pub(super) struct ProgressSourceDb {
    pub(super) source_id: crate::sample_sources::SourceId,
    pub(super) source_root: std::path::PathBuf,
    pub(super) conn: Connection,
}

/// Refresh the open source list on the poller's periodic cadence.
pub(super) fn refresh_sources(
    sources: &mut Vec<ProgressSourceDb>,
    last_refresh: &mut Instant,
    allowed_source_ids: Option<&HashSet<crate::sample_sources::SourceId>>,
) -> bool {
    if last_refresh.elapsed() < SOURCE_REFRESH_INTERVAL {
        return false;
    }
    *last_refresh = Instant::now();
    let Ok(state) = crate::sample_sources::library::load() else {
        return false;
    };
    let previous = std::mem::take(sources);
    let previous_len = previous.len();
    let mut reusable = previous
        .into_iter()
        .map(|source| {
            (
                (source.source_id.clone(), source.source_root.clone()),
                source.conn,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut next = Vec::new();
    for source in state.sources {
        if !source.root.is_dir() {
            continue;
        }
        if let Some(allowed) = allowed_source_ids
            && !allowed.contains(&source.id)
        {
            continue;
        }
        let conn = match reusable.remove(&(source.id.clone(), source.root.clone())) {
            Some(conn) => conn,
            None => match db::open_source_db_ui_read(&source.root) {
                Ok(conn) => conn,
                Err(_) => continue,
            },
        };
        next.push(ProgressSourceDb {
            source_id: source.id.clone(),
            source_root: source.root,
            conn,
        });
    }
    let changed = previous_len != next.len() || !reusable.is_empty();
    *sources = next;
    changed
}
