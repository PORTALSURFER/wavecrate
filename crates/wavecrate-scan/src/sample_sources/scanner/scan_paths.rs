use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::sample_sources::{SourceDatabase, WavEntry, is_supported_audio};

use super::{
    scan::{ScanContext, ScanError, ScanMode, ScanStats},
    scan_db_sync::db_sync_phase,
    scan_diff_phase::prepare_diff,
    scan_fs::ensure_root_dir,
    scan_walk::apply_prepared_chunk,
    scan_writer::{ScanWriter, UncoordinatedScanWriter},
};

const TARGET_PREPARE_BATCH_SIZE: usize = 64;

/// Reconcile a bounded set of changed paths against a source database.
///
/// This is the fast path for debounced watcher events. It only indexes rows at
/// or below the supplied relative paths, then applies the same diff and
/// pending-rename rules used by a normal quick scan.
pub fn sync_paths(db: &SourceDatabase, paths: &[PathBuf]) -> Result<ScanStats, ScanError> {
    sync_paths_with_progress(db, paths, None, &mut |_, _| {})
}

/// Reconcile changed paths with a progress callback and optional cancellation.
pub fn sync_paths_with_progress(
    db: &SourceDatabase,
    paths: &[PathBuf],
    cancel: Option<&AtomicBool>,
    on_progress: &mut impl FnMut(usize, &Path),
) -> Result<ScanStats, ScanError> {
    sync_paths_with_progress_and_writer(db, paths, cancel, on_progress, &UncoordinatedScanWriter)
}

/// Reconcile changed paths while coordinating only bounded database mutations.
pub fn sync_paths_with_progress_and_writer(
    db: &SourceDatabase,
    paths: &[PathBuf],
    cancel: Option<&AtomicBool>,
    on_progress: &mut impl FnMut(usize, &Path),
    writer: &impl ScanWriter,
) -> Result<ScanStats, ScanError> {
    let (manifest_revision, manifest_before) = super::manifest::capture_manifest_with_revision(db)?;
    let root = ensure_root_dir(db)?;
    let targets = collect_targets(db, &root, paths, cancel)?;
    let mut context = ScanContext::from_existing(
        targets.existing,
        ScanMode::Targeted,
        manifest_revision,
        manifest_before.clone(),
    );
    let mut prepared = Vec::with_capacity(TARGET_PREPARE_BATCH_SIZE);
    let mut committed = false;
    let result = (|| {
        for relative_path in targets.current_files {
            if let Some(cancel) = cancel
                && cancel.load(Ordering::Relaxed)
            {
                return Err(ScanError::Canceled);
            }
            let absolute = root.join(&relative_path);
            let prepared_file = match prepare_diff(&root, &absolute, &context) {
                Ok(prepared) => prepared,
                Err(error) if committed => {
                    if absolute.exists() {
                        context.existing.remove(&relative_path);
                    }
                    tracing::warn!(
                        path = %absolute.display(),
                        error = %error,
                        "Skipping targeted file after an earlier chunk committed"
                    );
                    continue;
                }
                Err(error) => return Err(error),
            };
            prepared.push(prepared_file);
            context.stats.total_files += 1;
            on_progress(context.stats.total_files, &absolute);
            if prepared.len() == TARGET_PREPARE_BATCH_SIZE {
                let chunk =
                    std::mem::replace(&mut prepared, Vec::with_capacity(TARGET_PREPARE_BATCH_SIZE));
                committed |= apply_prepared_chunk(
                    db,
                    &root,
                    cancel,
                    &mut context,
                    chunk,
                    committed,
                    writer,
                )?;
            }
        }
        if !prepared.is_empty() {
            let _ =
                apply_prepared_chunk(db, &root, cancel, &mut context, prepared, committed, writer)?;
        }
        db_sync_phase(db, &mut context, cancel, writer)
    })();
    super::scan::finish_scan_result(manifest_before, context, result)
}

struct TargetedScanTargets {
    current_files: BTreeSet<PathBuf>,
    existing: HashMap<PathBuf, WavEntry>,
}

fn collect_targets(
    db: &SourceDatabase,
    root: &Path,
    paths: &[PathBuf],
    cancel: Option<&AtomicBool>,
) -> Result<TargetedScanTargets, ScanError> {
    let mut current_files = BTreeSet::new();
    let mut existing = HashMap::new();
    for relative_path in normalized_targets(paths) {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        let absolute = root.join(&relative_path);
        collect_existing_rows(db, &relative_path, &mut existing)?;
        collect_current_files(&absolute, root, cancel, &mut current_files)?;
    }
    Ok(TargetedScanTargets {
        current_files,
        existing,
    })
}

fn normalized_targets(paths: &[PathBuf]) -> BTreeSet<PathBuf> {
    paths
        .iter()
        .filter(|path| !path.as_os_str().is_empty())
        .filter(|path| path.is_relative())
        .cloned()
        .collect()
}

fn collect_existing_rows(
    db: &SourceDatabase,
    relative_path: &Path,
    existing: &mut HashMap<PathBuf, WavEntry>,
) -> Result<(), ScanError> {
    for entry in db.list_files_under_path(relative_path)? {
        existing.entry(entry.relative_path.clone()).or_insert(entry);
    }
    Ok(())
}

fn collect_current_files(
    absolute_path: &Path,
    root: &Path,
    cancel: Option<&AtomicBool>,
    current_files: &mut BTreeSet<PathBuf>,
) -> Result<(), ScanError> {
    if absolute_path.is_dir() {
        collect_current_files_in_dir(absolute_path, root, cancel, current_files)?;
    } else if absolute_path.is_file() && is_supported_audio(absolute_path) {
        current_files.insert(strip_relative(root, absolute_path)?);
    }
    Ok(())
}

fn collect_current_files_in_dir(
    start_dir: &Path,
    root: &Path,
    cancel: Option<&AtomicBool>,
    current_files: &mut BTreeSet<PathBuf>,
) -> Result<(), ScanError> {
    let mut stack = vec![start_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(source) if dir != start_dir => {
                tracing::warn!(
                    dir = %dir.display(),
                    error = %source,
                    "Failed to read targeted sync directory"
                );
                continue;
            }
            Err(source) => {
                return Err(ScanError::Io { path: dir, source });
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(
                        dir = %dir.display(),
                        error = %err,
                        "Failed to read targeted sync directory entry"
                    );
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %err,
                        "Failed to read targeted sync file type"
                    );
                    continue;
                }
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() && is_supported_audio(&path) {
                current_files.insert(strip_relative(root, &path)?);
            }
        }
    }
    Ok(())
}

fn strip_relative(root: &Path, path: &Path) -> Result<PathBuf, ScanError> {
    path.strip_prefix(root)
        .map(PathBuf::from)
        .map_err(|_| ScanError::InvalidRoot(path.to_path_buf()))
}
