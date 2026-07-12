use super::*;
use crate::sample_sources::scanner::{self, ScanMode, ScanStats};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};

pub(super) fn launch_scan_worker(
    controller: &mut AppController,
    source: &SampleSource,
    mode: ScanMode,
    kind: ScanKind,
    paths: Option<Vec<PathBuf>>,
) {
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();
    let source_id = source.id.clone();
    controller
        .runtime
        .jobs
        .start_scan(source_id.clone(), rx, cancel.clone());
    let root = source.root.clone();
    std::thread::spawn(move || {
        let result = run_scan_worker(&root, mode, paths, cancel.as_ref(), |completed, path| {
            if completed == 1 || completed % 128 == 0 {
                let _ = tx.send(ScanJobMessage::Progress {
                    completed,
                    detail: Some(path.display().to_string()),
                });
            }
        });
        let _ = tx.send(ScanJobMessage::Finished(ScanResult {
            source_id,
            mode,
            kind,
            result,
        }));
    });
}

fn run_scan_worker(
    root: &Path,
    mode: ScanMode,
    paths: Option<Vec<PathBuf>>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, &Path),
) -> Result<ScanStats, scanner::ScanError> {
    let db = SourceDatabase::open_for_background_job(root)?;
    let stats = if mode == ScanMode::Targeted {
        scanner::sync_paths_with_progress(
            &db,
            &paths.unwrap_or_default(),
            Some(cancel),
            &mut progress,
        )?
    } else {
        scanner::scan_with_progress(&db, mode, Some(cancel), &mut progress)?
    };
    Ok(complete_deferred_hashes_preserving_committed(
        &db, stats, cancel,
    ))
}

fn complete_deferred_hashes_preserving_committed(
    db: &SourceDatabase,
    stats: ScanStats,
    cancel: &AtomicBool,
) -> ScanStats {
    let committed = stats.clone();
    match scanner::complete_deferred_hashes_with_cancel(db, stats, Some(cancel)) {
        Ok(completed) => completed,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "Deferred source hashing failed after scan changes were committed"
            );
            committed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn deferred_failure_preserves_committed_quick_scan_stats() {
        let root = tempfile::tempdir().expect("source");
        std::fs::write(root.path().join("large.wav"), vec![1_u8; 9 * 1024 * 1024])
            .expect("large wav");
        let db = SourceDatabase::open(root.path()).expect("source db");
        let stats = scanner::scan_once(&db).expect("quick scan");
        let cancel = AtomicBool::new(true);

        let completed = complete_deferred_hashes_preserving_committed(&db, stats, &cancel);

        assert_eq!(completed.added, 1);
        assert_eq!(completed.hashes_pending, 1);
        assert!(db.entry_for_path(Path::new("large.wav")).unwrap().is_some());
    }
}
