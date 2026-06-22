use std::path::PathBuf;

use wavecrate::sample_sources::{SourceDatabase, scanner};

use crate::native_app::app::SourceFilesystemSyncResult;

pub(super) fn sync_source_database_paths(
    source_id: String,
    root: PathBuf,
    paths: Vec<PathBuf>,
    changed_count: usize,
) -> SourceFilesystemSyncResult {
    let result = SourceDatabase::open_fast(&root)
        .map_err(|err| format!("open source index: {err}"))
        .and_then(|db| {
            scanner::sync_paths(&db, &paths).map_err(|err| format!("sync source index: {err}"))
        })
        .map(|_| ());
    SourceFilesystemSyncResult {
        source_id,
        changed_count,
        result,
    }
}
