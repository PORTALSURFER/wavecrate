use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use super::{
    super::{
        FileEntry, FolderEntry,
        collections::MissingCollectionSnapshot,
        path_helpers::{folder_label, path_id},
        scan_types::{
            FolderScanDiscovery, FolderScanItem, FolderScanProgress, FolderScanRequest,
            FolderScanResult, MetadataHydrationStatus,
        },
    },
    entry::{BrowserEntryKind, classify_path_without_following},
    file_entry_metadata::file_entry_with_snapshot_metadata,
    metadata::{SourceMetadataMap, source_browser_snapshot},
    traversal::placeholder_folder,
};
use wavecrate::sample_sources::{BrowserMetadataSnapshot, Rating, SourceDatabase, scanner};
use wavecrate_scan::{ScanStats, SourceTreeSnapshot};

struct CommittedSourceTreeSnapshot {
    revision: u64,
    layout: SourceTreeSnapshot,
}

/// Publish at most one source-index progress update per bounded file batch.
pub(in crate::native_app) const INDEX_PROGRESS_REPORT_INTERVAL: usize = 128;

#[cfg(test)]
pub(in crate::native_app) fn scan_source_with_progress(
    request: FolderScanRequest,
    progress: impl FnMut(FolderScanProgress),
    discovered: impl FnMut(FolderScanDiscovery),
) -> FolderScanResult {
    scan_source_with_progress_cancellable(request, progress, discovered, &AtomicBool::new(false))
}

pub(in crate::native_app) fn scan_source_with_progress_cancellable(
    request: FolderScanRequest,
    mut progress: impl FnMut(FolderScanProgress),
    mut discovered: impl FnMut(FolderScanDiscovery),
    cancel: &AtomicBool,
) -> FolderScanResult {
    let source_root_available =
        classify_path_without_following(&request.root) == Some(BrowserEntryKind::Directory);
    let (source_db_error, source_tree_snapshot) = if source_root_available {
        sync_source_database(&request, &mut progress, cancel)
    } else {
        (None, None)
    };
    let projection = if source_root_available && !cancel.load(Ordering::Acquire) {
        build_committed_projection(&request, source_tree_snapshot)
    } else {
        Err(String::from("source projection was not attempted"))
    };
    let (folder, ratings, metadata_hydration) = match projection {
        Ok((folder, ratings, revision)) => (
            folder,
            ratings,
            MetadataHydrationStatus::Complete { revision },
        ),
        Err(error) if source_root_available && !cancel.load(Ordering::Acquire) => (
            placeholder_folder(&request.root),
            SourceMetadataMap::new(),
            MetadataHydrationStatus::Failed { error },
        ),
        Err(_) => (
            placeholder_folder(&request.root),
            SourceMetadataMap::new(),
            MetadataHydrationStatus::NotAttempted,
        ),
    };
    let publish_discoveries = metadata_hydration.error().is_none();
    let mut scan = ScanProgressContext {
        request: &request,
        ratings,
        counter: ScanProgressCounter {
            completed: 0,
            files: 0,
            folders: 0,
        },
        progress: &mut progress,
        discovered: &mut discovered,
        cancel,
        publish_discoveries,
    };
    scan.report_initial();
    publish_projection(&folder, &mut scan);
    let missing_collection_snapshot =
        MissingCollectionSnapshot::from_source_metadata(&request.root, &folder, &scan.ratings);
    let file_count = scan.counter.files;
    let folder_count = scan.counter.folders;
    drop(scan);
    FolderScanResult {
        task_id: request.task_id,
        source_id: request.source_id,
        label: request.label,
        folder,
        missing_collection_snapshot,
        file_count,
        folder_count,
        source_db_error,
        metadata_hydration,
        source_root_available,
        cancelled: cancel.load(Ordering::Acquire),
    }
}

fn sync_source_database(
    request: &FolderScanRequest,
    progress: &mut impl FnMut(FolderScanProgress),
    cancel: &AtomicBool,
) -> (Option<String>, Option<CommittedSourceTreeSnapshot>) {
    let db = match SourceDatabase::open_for_background_job_with_database_root(
        &request.root,
        &request.database_root,
    ) {
        Ok(db) => db,
        Err(err) => return (Some(format!("open source index: {err}")), None),
    };
    let mut sync_progress = |completed: usize, path: &Path| {
        if completed != 1 && !completed.is_multiple_of(INDEX_PROGRESS_REPORT_INTERVAL) {
            return;
        }
        progress(FolderScanProgress {
            task_id: request.task_id,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Indexing"),
            completed,
            total: 0,
            detail: path.display().to_string(),
        });
    };
    let stats = match scanner::scan_with_progress(
        &db,
        scanner::ScanMode::Quick,
        Some(cancel),
        &mut sync_progress,
    ) {
        Ok(stats) => stats,
        Err(err) => return (Some(format!("sync source index: {err}")), None),
    };
    let fallback_snapshot =
        stats
            .source_tree_snapshot
            .clone()
            .map(|layout| CommittedSourceTreeSnapshot {
                revision: stats.committed_delta.revision,
                layout,
            });
    match scanner::complete_deferred_rename_candidates_with_cancel(&db, stats, Some(cancel)) {
        Ok(ScanStats {
            committed_delta,
            source_tree_snapshot,
            ..
        }) => (
            None,
            source_tree_snapshot.map(|layout| CommittedSourceTreeSnapshot {
                revision: committed_delta.revision,
                layout,
            }),
        ),
        Err(err) => (
            Some(format!("finish deferred rename hashing: {err}")),
            fallback_snapshot,
        ),
    }
}

#[derive(Default)]
struct ProjectionFolder {
    children: Vec<PathBuf>,
    files: Vec<FileEntry>,
}

fn build_committed_projection(
    request: &FolderScanRequest,
    source_tree_snapshot: Option<CommittedSourceTreeSnapshot>,
) -> Result<(FolderEntry, SourceMetadataMap, u64), String> {
    let started_at = Instant::now();
    let committed = source_tree_snapshot
        .ok_or_else(|| String::from("authoritative source traversal did not produce a layout"))?;
    let layout = committed.layout;
    if !layout.is_complete() {
        return Err(format!(
            "authoritative source traversal was incomplete: {}",
            layout.diagnostics.join("; ")
        ));
    }
    let BrowserMetadataSnapshot { revision, files } =
        source_browser_snapshot(&request.root, &request.database_root)
            .map_err(|error| error.to_string())?;
    if revision != committed.revision {
        return Err(format!(
            "browser snapshot revision {revision} did not match authoritative traversal revision {}",
            committed.revision
        ));
    }
    let ratings = files
        .iter()
        .map(|entry| {
            (
                entry.relative_path.clone(),
                (
                    entry.rating,
                    entry.locked,
                    entry.collections.clone(),
                    entry.last_played_at,
                    entry.last_curated_at,
                ),
            )
        })
        .collect::<SourceMetadataMap>();

    let mut folders = layout
        .directories
        .iter()
        .cloned()
        .map(|path| (path, ProjectionFolder::default()))
        .collect::<BTreeMap<_, _>>();
    folders.entry(PathBuf::new()).or_default();
    for directory in layout
        .directories
        .iter()
        .filter(|path| !path.as_os_str().is_empty())
    {
        let parent = directory.parent().unwrap_or_else(|| Path::new(""));
        folders
            .entry(parent.to_path_buf())
            .or_default()
            .children
            .push(directory.clone());
    }
    for entry in files.iter().filter(|entry| !entry.missing) {
        let absolute = request.root.join(&entry.relative_path);
        let file = file_entry_with_snapshot_metadata(
            &absolute,
            entry.file_size,
            entry.rating,
            entry.locked,
            entry.collections.clone(),
            entry.last_played_at,
            entry.last_curated_at,
        );
        folders
            .entry(
                entry
                    .relative_path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_path_buf(),
            )
            .or_default()
            .files
            .push(file);
    }
    let other_file_count = layout.other_files.len();
    for entry in layout.other_files {
        let absolute = request.root.join(&entry.relative_path);
        let file = file_entry_with_snapshot_metadata(
            &absolute,
            entry.file_size,
            Rating::NEUTRAL,
            false,
            Vec::new(),
            None,
            None,
        );
        let parent = entry
            .relative_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        let destination = folders.entry(parent).or_default();
        if !destination
            .files
            .iter()
            .any(|existing| existing.id == file.id)
        {
            destination.files.push(file);
        }
    }
    for folder in folders.values_mut() {
        folder.children.sort_by(|left, right| {
            folder_label(&request.root.join(left))
                .to_ascii_lowercase()
                .cmp(&folder_label(&request.root.join(right)).to_ascii_lowercase())
        });
        folder.files.sort_by_key(FileEntry::name_sort_key);
    }
    let folder_count = folders.len();
    let file_count = files.iter().filter(|entry| !entry.missing).count() + other_file_count;
    let folder = materialize_projection_folder(&request.root, Path::new(""), &folders);
    tracing::info!(
        source_id = request.source_id,
        revision,
        filesystem_traversals = 1,
        sqlite_snapshots = 1,
        folder_count,
        file_count,
        elapsed_ms = started_at.elapsed().as_millis(),
        "Built browser projection from committed source snapshot"
    );
    Ok((folder, ratings, revision))
}

fn materialize_projection_folder(
    root: &Path,
    relative: &Path,
    folders: &BTreeMap<PathBuf, ProjectionFolder>,
) -> FolderEntry {
    let absolute = if relative.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    };
    let projection = folders.get(relative);
    FolderEntry {
        id: path_id(&absolute),
        name: folder_label(&absolute),
        children: projection
            .map(|folder| {
                folder
                    .children
                    .iter()
                    .map(|child| materialize_projection_folder(root, child, folders))
                    .collect()
            })
            .unwrap_or_default(),
        files: projection
            .map(|folder| folder.files.clone())
            .unwrap_or_default(),
    }
}

struct ScanProgressCounter {
    completed: usize,
    files: usize,
    folders: usize,
}

struct ScanProgressContext<'a, P, D>
where
    P: FnMut(FolderScanProgress),
    D: FnMut(FolderScanDiscovery),
{
    request: &'a FolderScanRequest,
    ratings: SourceMetadataMap,
    counter: ScanProgressCounter,
    progress: &'a mut P,
    discovered: &'a mut D,
    cancel: &'a AtomicBool,
    publish_discoveries: bool,
}

impl<P, D> ScanProgressContext<'_, P, D>
where
    P: FnMut(FolderScanProgress),
    D: FnMut(FolderScanDiscovery),
{
    fn report_initial(&mut self) {
        (self.progress)(FolderScanProgress {
            task_id: self.request.task_id,
            source_id: self.request.source_id.clone(),
            label: self.request.label.clone(),
            phase: String::from("Scanning"),
            completed: 0,
            total: 0,
            detail: self.request.root.display().to_string(),
        });
    }

    fn record_folder(&mut self, path: &Path, parent_id: &str) {
        self.counter.completed += 1;
        self.counter.folders += 1;
        self.maybe_report_progress(path);
        if self.publish_discoveries {
            (self.discovered)(FolderScanDiscovery {
                task_id: self.request.task_id,
                source_id: self.request.source_id.clone(),
                parent_id: parent_id.to_string(),
                item: FolderScanItem::Folder(placeholder_folder(path)),
            });
        }
    }

    fn record_folder_snapshot_start(&mut self, folder_id: &str) {
        if self.publish_discoveries {
            (self.discovered)(FolderScanDiscovery {
                task_id: self.request.task_id,
                source_id: self.request.source_id.clone(),
                parent_id: folder_id.to_string(),
                item: FolderScanItem::ResetFolder,
            });
        }
    }

    fn record_file(&mut self, path: &Path, parent_id: &str, file: FileEntry) {
        self.counter.completed += 1;
        self.counter.files += 1;
        self.maybe_report_progress(path);
        if self.publish_discoveries {
            (self.discovered)(FolderScanDiscovery {
                task_id: self.request.task_id,
                source_id: self.request.source_id.clone(),
                parent_id: parent_id.to_string(),
                item: FolderScanItem::File(file),
            });
        }
    }

    fn maybe_report_progress(&mut self, path: &Path) {
        if self.counter.completed == 1 || self.counter.completed.is_multiple_of(64) {
            (self.progress)(FolderScanProgress {
                task_id: self.request.task_id,
                source_id: self.request.source_id.clone(),
                label: self.request.label.clone(),
                phase: String::from("Scanning"),
                completed: self.counter.completed,
                total: 0,
                detail: path.display().to_string(),
            });
        }
    }
}

fn publish_projection<P, D>(folder: &FolderEntry, scan: &mut ScanProgressContext<'_, P, D>)
where
    P: FnMut(FolderScanProgress),
    D: FnMut(FolderScanDiscovery),
{
    if scan.cancel.load(Ordering::Acquire) {
        return;
    }
    let path = PathBuf::from(&folder.id);
    let parent_id = folder.id.clone();
    scan.record_folder_snapshot_start(&parent_id);
    for child in &folder.children {
        if scan.cancel.load(Ordering::Acquire) {
            return;
        }
        let child_path = PathBuf::from(&child.id);
        scan.record_folder(&child_path, &parent_id);
        publish_projection(child, scan);
    }
    for file in &folder.files {
        if scan.cancel.load(Ordering::Acquire) {
            return;
        }
        scan.record_file(&path.join(&file.name), &parent_id, file.clone());
    }
}
