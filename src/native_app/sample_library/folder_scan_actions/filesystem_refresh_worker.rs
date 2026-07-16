use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use wavecrate::sample_sources::{SourceDatabase, scanner};

use crate::native_app::app::{SourceFilesystemSyncResult, SourceFilesystemSyncSuccess};

pub(super) fn sync_source_database_paths(
    source_id: String,
    root: PathBuf,
    database_root: PathBuf,
    paths: Vec<PathBuf>,
    changed_count: usize,
    cancel: &AtomicBool,
) -> SourceFilesystemSyncResult {
    let result = SourceDatabase::open_for_background_job_with_database_root(&root, &database_root)
        .map_err(|err| format!("open source index: {err}"))
        .and_then(|db| {
            let stats =
                scanner::sync_paths_with_progress(&db, &paths, Some(cancel), &mut |_, _| {})
                    .map_err(|err| format!("sync source index: {err}"))?;
            let committed = stats.clone();
            let completed = match scanner::complete_deferred_rename_candidates_with_cancel(
                &db,
                stats,
                Some(cancel),
            ) {
                Ok(completed) => completed,
                Err(scanner::ScanError::Canceled) => committed,
                Err(error) => {
                    tracing::warn!(
                        source_id,
                        error = %error,
                        "Deferred rename reconciliation failed after filesystem sync committed"
                    );
                    committed
                }
            };
            Ok(SourceFilesystemSyncSuccess {
                renames_reconciled: completed.renames_reconciled,
            })
        });
    SourceFilesystemSyncResult {
        source_id,
        changed_count,
        cancelled: cancel.load(Ordering::Acquire),
        result,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
    };

    use wavecrate::sample_sources::{Rating, SourceDatabase, scanner};

    use super::{SourceFilesystemSyncSuccess, sync_source_database_paths};

    #[test]
    fn filesystem_sync_returns_deferred_rename_results_for_refresh() {
        let root = tempfile::tempdir().expect("source root");
        let old = root.path().join("old.wav");
        let new = root.path().join("new.wav");
        std::fs::write(&old, vec![5_u8; 9 * 1024 * 1024]).expect("large wav");
        let db = SourceDatabase::open(root.path()).expect("source db");
        scanner::hard_rescan(&db).expect("initial scan");
        db.set_tag(Path::new("old.wav"), Rating::KEEP_1)
            .expect("tag old path");
        std::fs::rename(&old, &new).expect("rename wav");

        let result = sync_source_database_paths(
            String::from("source-a"),
            root.path().to_path_buf(),
            root.path().to_path_buf(),
            vec![PathBuf::from("old.wav"), PathBuf::from("new.wav")],
            2,
            &AtomicBool::new(false),
        );

        assert_eq!(
            result.result,
            Ok(SourceFilesystemSyncSuccess {
                renames_reconciled: 1
            })
        );
        assert_eq!(
            db.entry_for_path(Path::new("new.wav"))
                .unwrap()
                .unwrap()
                .tag,
            Rating::KEEP_1
        );
    }

    #[test]
    fn filesystem_sync_leaves_non_rename_hashing_for_the_supervisor() {
        let root = tempfile::tempdir().expect("source root");
        let fresh = root.path().join("fresh.wav");
        std::fs::write(&fresh, vec![7_u8; 9 * 1024 * 1024]).expect("large wav");

        let result = sync_source_database_paths(
            String::from("source-a"),
            root.path().to_path_buf(),
            root.path().to_path_buf(),
            vec![PathBuf::from("fresh.wav")],
            1,
            &AtomicBool::new(false),
        );

        assert_eq!(
            result.result,
            Ok(SourceFilesystemSyncSuccess {
                renames_reconciled: 0
            })
        );
        let db = SourceDatabase::open(root.path()).expect("source db");
        assert!(
            db.entry_for_path(Path::new("fresh.wav"))
                .expect("read entry")
                .expect("fresh entry")
                .content_hash
                .is_none(),
            "ordinary deep hashing must remain queued for the supervisor"
        );
    }

    #[test]
    fn filesystem_sync_reports_lifecycle_cancellation_for_requeue() {
        let root = tempfile::tempdir().expect("source root");
        let fresh = root.path().join("fresh.wav");
        std::fs::write(&fresh, b"fresh").expect("wav");
        let cancel = AtomicBool::new(true);

        let result = sync_source_database_paths(
            String::from("source-a"),
            root.path().to_path_buf(),
            root.path().to_path_buf(),
            vec![PathBuf::from("fresh.wav")],
            1,
            &cancel,
        );

        assert!(result.cancelled);
        assert!(result.result.is_err());
        let db = SourceDatabase::open(root.path()).expect("source db");
        assert!(
            db.entry_for_path(Path::new("fresh.wav"))
                .expect("read entry")
                .is_none()
        );
    }
}
