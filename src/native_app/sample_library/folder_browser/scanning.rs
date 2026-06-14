use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use super::{
    FileEntry, FolderEntry,
    path_helpers::{file_label, folder_label, path_id},
    scan_types::{
        FolderScanDiscovery, FolderScanItem, FolderScanProgress, FolderScanRequest,
        FolderScanResult, FolderVerifyOutcome, FolderVerifyRequest, FolderVerifyResult,
        FolderVerifySnapshot,
    },
};
use wavecrate::sample_sources::{Rating, SampleCollection, SourceDatabase};

mod discovery_merge;
mod file_entry_metadata;
pub(super) use discovery_merge::{merge_scan_discovery, upsert_file, upsert_folder};
pub(super) use file_entry_metadata::file_entry;
use file_entry_metadata::file_entry_with_metadata;

pub(super) fn default_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

pub(super) fn load_root_folder(root: PathBuf) -> FolderEntry {
    let ratings = source_rating_map(&root);
    load_folder(&root, &root, &ratings).unwrap_or_else(|| FolderEntry {
        id: path_id(&root),
        name: folder_label(&root),
        children: Vec::new(),
        files: Vec::new(),
    })
}

pub(super) fn placeholder_folder(root: &Path) -> FolderEntry {
    FolderEntry {
        id: path_id(root),
        name: folder_label(root),
        children: Vec::new(),
        files: Vec::new(),
    }
}

pub(in crate::native_app) fn verify_direct_folder(
    request: FolderVerifyRequest,
) -> FolderVerifyResult {
    let outcome = match read_direct_folder_snapshot(&request.folder_path) {
        DirectFolderSnapshot::Missing => FolderVerifyOutcome::Missing,
        DirectFolderSnapshot::Unavailable => FolderVerifyOutcome::Unchanged,
        DirectFolderSnapshot::Available(snapshot) => {
            if direct_folder_changed(&request, &snapshot) {
                FolderVerifyOutcome::Changed(snapshot)
            } else {
                FolderVerifyOutcome::Unchanged
            }
        }
    };
    FolderVerifyResult {
        source_id: request.source_id,
        folder_path: request.folder_path,
        outcome,
    }
}

pub(in crate::native_app) fn scan_source_with_progress(
    request: FolderScanRequest,
    mut progress: impl FnMut(FolderScanProgress),
    mut discovered: impl FnMut(FolderScanDiscovery),
) -> FolderScanResult {
    let ratings = source_rating_map(&request.root);
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
    };
    scan.report_initial();
    let folder = load_folder_with_progress(&request.root, &mut scan)
        .unwrap_or_else(|| placeholder_folder(&request.root));
    let file_count = scan.counter.files;
    let folder_count = scan.counter.folders;
    drop(scan);
    discovered(FolderScanDiscovery {
        task_id: request.task_id,
        source_id: request.source_id.clone(),
        parent_id: path_id(&request.root),
        item: FolderScanItem::CompletedFolder(folder.clone()),
    });
    FolderScanResult {
        task_id: request.task_id,
        source_id: request.source_id,
        label: request.label,
        folder,
        file_count,
        folder_count,
    }
}

type SourceMetadataMap = HashMap<PathBuf, (Rating, bool, Vec<SampleCollection>, Option<i64>)>;

fn source_rating_map(root: &Path) -> SourceMetadataMap {
    let Ok(db) = SourceDatabase::open_read_only(root) else {
        return HashMap::new();
    };
    let Ok(entries) = db.list_files() else {
        return HashMap::new();
    };
    entries
        .into_iter()
        .map(|entry| {
            let collections = db
                .collections_for_path(&entry.relative_path)
                .unwrap_or_default();
            (
                entry.relative_path,
                (entry.tag, entry.locked, collections, entry.last_played_at),
            )
        })
        .collect()
}

fn load_folder(
    path: &Path,
    source_root: &Path,
    ratings: &SourceMetadataMap,
) -> Option<FolderEntry> {
    let entries = read_sorted_entries(path)?;
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .filter_map(|entry| load_folder(entry, source_root, ratings))
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(|entry| rated_file_entry(entry, source_root, ratings))
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

enum DirectFolderSnapshot {
    Available(FolderVerifySnapshot),
    Missing,
    Unavailable,
}

fn read_direct_folder_snapshot(path: &Path) -> DirectFolderSnapshot {
    if !path.is_dir() {
        return DirectFolderSnapshot::Missing;
    }
    let Some(entries) = read_sorted_entries(path) else {
        return DirectFolderSnapshot::Unavailable;
    };
    let child_paths = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .cloned()
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(file_entry)
        .collect::<Vec<_>>();
    DirectFolderSnapshot::Available(FolderVerifySnapshot { child_paths, files })
}

fn direct_folder_changed(request: &FolderVerifyRequest, snapshot: &FolderVerifySnapshot) -> bool {
    let child_ids = snapshot
        .child_paths
        .iter()
        .map(|path| path_id(path))
        .collect::<Vec<_>>();
    if child_ids != request.cached_child_ids {
        return true;
    }
    let file_signatures = snapshot
        .files
        .iter()
        .map(|file| (file.id.clone(), file.size_bytes))
        .collect::<Vec<_>>();
    file_signatures != request.cached_file_signatures
}

pub(super) fn load_folder_at_path(path: &Path, source_root: &Path) -> Option<FolderEntry> {
    let ratings = source_rating_map(source_root);
    load_folder(path, source_root, &ratings)
}

pub(super) fn file_entry_for_source_path(path: &PathBuf, source_root: &Path) -> FileEntry {
    let metadata = source_file_metadata(source_root, path).unwrap_or((
        Rating::NEUTRAL,
        false,
        Vec::new(),
        None,
    ));
    file_entry_with_metadata(path, metadata.0, metadata.1, metadata.2, metadata.3)
}

fn source_file_metadata(
    source_root: &Path,
    path: &Path,
) -> Option<(Rating, bool, Vec<SampleCollection>, Option<i64>)> {
    let relative = path.strip_prefix(source_root).ok()?;
    let db = SourceDatabase::open_read_only(source_root).ok()?;
    let entry = db.entry_for_path(relative).ok()??;
    let collections = db.collections_for_path(relative).unwrap_or_default();
    Some((entry.tag, entry.locked, collections, entry.last_played_at))
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
        (self.discovered)(FolderScanDiscovery {
            task_id: self.request.task_id,
            source_id: self.request.source_id.clone(),
            parent_id: parent_id.to_string(),
            item: FolderScanItem::Folder(placeholder_folder(path)),
        });
    }

    fn record_file(&mut self, path: &Path, parent_id: &str, file: FileEntry) {
        self.counter.completed += 1;
        self.counter.files += 1;
        self.maybe_report_progress(path);
        (self.discovered)(FolderScanDiscovery {
            task_id: self.request.task_id,
            source_id: self.request.source_id.clone(),
            parent_id: parent_id.to_string(),
            item: FolderScanItem::File(file),
        });
    }

    fn record_completed_folder(&mut self, parent_id: &str, folder: FolderEntry) {
        (self.discovered)(FolderScanDiscovery {
            task_id: self.request.task_id,
            source_id: self.request.source_id.clone(),
            parent_id: parent_id.to_string(),
            item: FolderScanItem::CompletedFolder(folder),
        });
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

fn load_folder_with_progress<P, D>(
    path: &Path,
    scan: &mut ScanProgressContext<'_, P, D>,
) -> Option<FolderEntry>
where
    P: FnMut(FolderScanProgress),
    D: FnMut(FolderScanDiscovery),
{
    let entries = read_sorted_entries(path)?;
    let parent_id = path_id(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .filter_map(|entry| {
            scan.record_folder(entry, &parent_id);
            let child = load_folder_with_progress(entry, scan)?;
            scan.record_completed_folder(&parent_id, child.clone());
            Some(child)
        })
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(|entry| {
            let file = rated_file_entry(entry, &scan.request.root, &scan.ratings);
            scan.record_file(entry, &parent_id, file.clone());
            file
        })
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

fn rated_file_entry(path: &PathBuf, source_root: &Path, ratings: &SourceMetadataMap) -> FileEntry {
    let (rating, locked, collections, last_played_at) = path
        .strip_prefix(source_root)
        .ok()
        .and_then(|relative| ratings.get(relative).cloned())
        .unwrap_or((Rating::NEUTRAL, false, Vec::new(), None));
    file_entry_with_metadata(path, rating, locked, collections, last_played_at)
}

fn read_sorted_entries(path: &Path) -> Option<Vec<PathBuf>> {
    let read_dir = fs::read_dir(path).ok()?;
    let mut entries = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        file_label(a)
            .to_ascii_lowercase()
            .cmp(&file_label(b).to_ascii_lowercase())
    });
    Some(entries)
}
