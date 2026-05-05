use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::SourceWriteBatch;

use super::scan::{ScanContext, ScanError};
use super::scan_diff::apply_diff;
use super::scan_fs::read_facts;

pub(super) fn diff_phase(
    db: &SourceDatabase,
    batch: &mut SourceWriteBatch<'_>,
    root: &Path,
    path: &Path,
    context: &mut ScanContext,
    cancel: Option<&AtomicBool>,
) -> Result<(), ScanError> {
    let facts = read_facts(root, path)?;
    apply_diff(db, batch, facts, context, root, cancel)?;
    context.stats.total_files += 1;
    Ok(())
}
