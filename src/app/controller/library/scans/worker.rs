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
    controller.runtime.jobs.start_scan(rx, cancel.clone());
    let source_id = source.id.clone();
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
    if stats.hashes_pending > 0 {
        scanner::schedule_deep_hash_scan(root.to_path_buf());
    }
    Ok(stats)
}
