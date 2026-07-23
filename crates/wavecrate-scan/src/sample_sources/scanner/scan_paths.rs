use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, ambient_authority};
use cap_std::fs::{Dir, OpenOptions};

use crate::sample_sources::{SourceDatabase, WavEntry, is_supported_audio};

use super::{
    scan::{ScanContext, ScanError, ScanMode, ScanStats},
    scan_db_sync::db_sync_phase,
    scan_diff_phase::prepare_diff_from_facts,
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
        for targeted_file in targets.current_files {
            if let Some(cancel) = cancel
                && cancel.load(Ordering::Relaxed)
            {
                return Err(ScanError::Canceled);
            }
            let absolute = root.join(&targeted_file.relative);
            let mut prepared_file = prepare_diff_from_facts(targeted_file.facts, &context);
            prepared_file.targeted_file = Some(targeted_file.file);
            prepared_file.targeted_handle_verified = true;
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
        let committed_snapshot = db_sync_phase(db, &mut context, cancel, writer)?;
        super::scan::reconcile_scan_renames(
            db,
            &mut context,
            &manifest_before,
            committed_snapshot,
            cancel,
            writer,
        )
    })();
    super::scan::finish_scan_result(manifest_before, context, result)
}

struct TargetedScanTargets {
    current_files: Vec<TargetedFile>,
    existing: HashMap<PathBuf, WavEntry>,
}

struct TargetedFile {
    relative: PathBuf,
    facts: super::scan_fs::FileFacts,
    file: std::fs::File,
}

fn collect_targets(
    db: &SourceDatabase,
    root: &Path,
    paths: &[PathBuf],
    cancel: Option<&AtomicBool>,
) -> Result<TargetedScanTargets, ScanError> {
    let source_root =
        Dir::open_ambient_dir(root, ambient_authority()).map_err(|source| ScanError::Io {
            path: root.to_path_buf(),
            source,
        })?;
    let mut current_files = BTreeMap::new();
    let mut existing = HashMap::new();
    for relative_path in normalized_targets(paths) {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        collect_existing_rows(db, &relative_path, &mut existing)?;
        collect_current_files(
            &source_root,
            root,
            &relative_path,
            cancel,
            &mut current_files,
        )?;
    }
    Ok(TargetedScanTargets {
        current_files: current_files.into_values().collect(),
        existing,
    })
}

fn normalized_targets(paths: &[PathBuf]) -> BTreeSet<PathBuf> {
    paths
        .iter()
        .filter(|path| !path.as_os_str().is_empty())
        .filter(|path| {
            path.is_relative()
                && path
                    .components()
                    .all(|component| matches!(component, Component::CurDir | Component::Normal(_)))
        })
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
    source_root: &Dir,
    root: &Path,
    relative_path: &Path,
    cancel: Option<&AtomicBool>,
    current_files: &mut BTreeMap<PathBuf, TargetedFile>,
) -> Result<(), ScanError> {
    let Some((parent, name)) = open_target_parent(source_root, root, relative_path)? else {
        return Ok(());
    };
    let absolute_path = root.join(relative_path);
    let name_path = Path::new(&name);
    let metadata = match parent.symlink_metadata(name_path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(ScanError::Io {
                path: absolute_path,
                source,
            });
        }
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Ok(());
    }
    if file_type.is_dir() {
        let dir = match parent.open_dir_nofollow(name_path) {
            Ok(dir) => dir,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(ScanError::Io {
                    path: root.join(relative_path),
                    source,
                });
            }
        };
        collect_current_files_in_dir(dir, root, relative_path, cancel, current_files)?;
    } else if file_type.is_file() {
        collect_current_file(
            &parent,
            Path::new(&name),
            root,
            relative_path,
            current_files,
        )?;
    }
    Ok(())
}

fn open_target_parent(
    source_root: &Dir,
    root: &Path,
    relative_path: &Path,
) -> Result<Option<(Dir, std::ffi::OsString)>, ScanError> {
    let Some(name) = relative_path.file_name() else {
        return Ok(None);
    };
    let mut dir = source_root.try_clone().map_err(|source| ScanError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let mut traversed = PathBuf::new();
    for component in relative_path
        .parent()
        .into_iter()
        .flat_map(Path::components)
    {
        let Component::Normal(part) = component else {
            continue;
        };
        traversed.push(part);
        dir = match dir.open_dir_nofollow(part) {
            Ok(dir) => dir,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                let path = root.join(&traversed);
                if dir
                    .symlink_metadata(part)
                    .is_ok_and(|metadata| metadata.is_symlink())
                {
                    return Ok(None);
                }
                return Err(ScanError::Io { path, source });
            }
        };
    }
    Ok(Some((dir, name.to_os_string())))
}

fn collect_current_files_in_dir(
    start_dir: Dir,
    root: &Path,
    start_relative: &Path,
    cancel: Option<&AtomicBool>,
    current_files: &mut BTreeMap<PathBuf, TargetedFile>,
) -> Result<(), ScanError> {
    let mut stack = vec![(start_dir, start_relative.to_path_buf())];
    while let Some((dir, relative_dir)) = stack.pop() {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        let entries = match dir.entries() {
            Ok(entries) => entries,
            Err(source) if relative_dir != start_relative => {
                tracing::warn!(
                    dir = %root.join(&relative_dir).display(),
                    error = %source,
                    "Failed to read targeted sync directory"
                );
                continue;
            }
            Err(source) => {
                return Err(ScanError::Io {
                    path: root.join(&relative_dir),
                    source,
                });
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(
                        dir = %root.join(&relative_dir).display(),
                        error = %err,
                        "Failed to read targeted sync directory entry"
                    );
                    continue;
                }
            };
            let name = entry.file_name();
            let name_path = Path::new(&name);
            let path = root.join(&relative_dir).join(name_path);
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
                let child_dir = match dir.open_dir_nofollow(name_path) {
                    Ok(child_dir) => child_dir,
                    Err(source) if source.kind() == io::ErrorKind::NotFound => continue,
                    Err(source) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %source,
                            "Failed to open targeted sync directory without following links"
                        );
                        continue;
                    }
                };
                stack.push((child_dir, relative_dir.join(name_path)));
            } else if file_type.is_file() {
                let relative_path = relative_dir.join(name_path);
                collect_current_file(&dir, name_path, root, &relative_path, current_files)?;
            }
        }
    }
    Ok(())
}

fn collect_current_file(
    parent: &Dir,
    name: &Path,
    root: &Path,
    relative_path: &Path,
    current_files: &mut BTreeMap<PathBuf, TargetedFile>,
) -> Result<(), ScanError> {
    let absolute_path = root.join(relative_path);
    if !is_supported_audio(&absolute_path) || has_hidden_ancestor(relative_path) {
        return Ok(());
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = match parent.open_with(name, &options) {
        Ok(file) => file.into_std(),
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            // The no-follow open is the source-boundary decision. A file that
            // became a link after directory enumeration is rejected here.
            tracing::warn!(
                path = %absolute_path.display(),
                error = %source,
                "Skipping targeted sync file that could not be opened without following links"
            );
            return Ok(());
        }
    };
    let facts = super::scan_fs::read_facts_from_open_file(root, &absolute_path, &file)?;
    current_files
        .entry(relative_path.to_path_buf())
        .or_insert(TargetedFile {
            relative: relative_path.to_path_buf(),
            facts,
            file,
        });
    Ok(())
}

fn has_hidden_ancestor(relative_path: &Path) -> bool {
    relative_path.parent().is_some_and(|parent| {
        parent.components().any(|component| {
            let Component::Normal(name) = component else {
                return false;
            };
            name.to_str().is_some_and(|name| name.starts_with('.'))
        })
    })
}
