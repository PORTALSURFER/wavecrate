use std::path::PathBuf;

use wavecrate::sample_sources::{SourceDatabase, scanner};

use crate::native_app::app::SourceFilesystemSyncResult;

pub(super) fn sync_source_database_paths(
    source_id: String,
    root: PathBuf,
    database_root: PathBuf,
    paths: Vec<PathBuf>,
    changed_count: usize,
) -> SourceFilesystemSyncResult {
    let result = SourceDatabase::open_for_background_job_with_database_root(&root, &database_root)
        .map_err(|err| format!("open source index: {err}"))
        .and_then(|db| {
            let stats = scanner::sync_paths(&db, &paths)
                .map_err(|err| format!("sync source index: {err}"))?;
            if stats.hashes_pending > 0 {
                scanner::schedule_deep_hash_scan_with_database_root(
                    root.clone(),
                    database_root.clone(),
                );
            }
            Ok(())
        });
    SourceFilesystemSyncResult {
        source_id,
        changed_count,
        result,
    }
}
