use std::time::{SystemTime, UNIX_EPOCH};

use super::scan::{ScanContext, ScanError, ScanMode};
use super::scan_diff::mark_missing;
use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::META_LAST_SCAN_COMPLETED_AT;

const MISSING_BATCH_SIZE: usize = 64;

pub(super) fn db_sync_phase(
    db: &SourceDatabase,
    context: &mut ScanContext,
) -> Result<(), ScanError> {
    let mut existing = std::mem::take(&mut context.existing)
        .into_values()
        .collect::<Vec<_>>()
        .into_iter();
    loop {
        let chunk = existing
            .by_ref()
            .take(MISSING_BATCH_SIZE)
            .collect::<Vec<_>>();
        if chunk.is_empty() {
            break;
        }
        let mut batch = db.write_batch()?;
        context.ensure_rename_candidate_generation(&mut batch)?;
        mark_missing(db, &mut batch, chunk, &mut context.stats, context.mode)?;
        batch.commit()?;
    }

    let mut batch = db.write_batch()?;
    context.ensure_rename_candidate_generation(&mut batch)?;
    if context.mode == ScanMode::Hard {
        batch.clear_all_pending_renames()?;
        batch.clear_all_pending_rename_destinations()?;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    batch.set_metadata(META_LAST_SCAN_COMPLETED_AT, &timestamp)?;
    batch.commit()?;
    Ok(())
}
