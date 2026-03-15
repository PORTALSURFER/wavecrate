use crate::app::controller::library::analysis_jobs::db;
use rusqlite::Connection;
use std::time::Instant;

use super::SOURCE_REFRESH_INTERVAL;

/// Open source database handle tracked by the progress poller.
pub(super) struct ProgressSourceDb {
    pub(super) source_id: crate::sample_sources::SourceId,
    pub(super) conn: Connection,
}

/// Refresh the open source list on the poller's periodic cadence.
pub(super) fn refresh_sources(
    sources: &mut Vec<ProgressSourceDb>,
    last_refresh: &mut Instant,
    allowed_source_ids: Option<&std::collections::HashSet<crate::sample_sources::SourceId>>,
) {
    if last_refresh.elapsed() < SOURCE_REFRESH_INTERVAL {
        return;
    }
    *last_refresh = Instant::now();
    let Ok(state) = crate::sample_sources::library::load() else {
        return;
    };
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
        let conn = match db::open_source_db(&source.root) {
            Ok(conn) => conn,
            Err(_) => continue,
        };
        next.push(ProgressSourceDb {
            source_id: source.id.clone(),
            conn,
        });
    }
    *sources = next;
}
