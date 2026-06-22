use std::path::Path;

use super::{
    super::{
        FileEntry, FolderEntry,
        collections::MissingCollectionSnapshot,
        path_helpers::{folder_label, path_id},
        scan_types::{
            FolderScanDiscovery, FolderScanItem, FolderScanProgress, FolderScanRequest,
            FolderScanResult,
        },
    },
    metadata::{SourceMetadataMap, rated_file_entry, source_rating_map},
    traversal::{placeholder_folder, read_sorted_entries},
};
use wavecrate::sample_sources::{SourceDatabase, scanner};

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
    let missing_collection_snapshot =
        MissingCollectionSnapshot::from_source_metadata(&request.root, &folder, &scan.ratings);
    let file_count = scan.counter.files;
    let folder_count = scan.counter.folders;
    drop(scan);
    let source_db_error = sync_source_database(&request, &mut progress);
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
        missing_collection_snapshot,
        file_count,
        folder_count,
        source_db_error,
    }
}

fn sync_source_database(
    request: &FolderScanRequest,
    progress: &mut impl FnMut(FolderScanProgress),
) -> Option<String> {
    let db = match SourceDatabase::open_fast(&request.root) {
        Ok(db) => db,
        Err(err) => return Some(format!("open source index: {err}")),
    };
    let mut sync_progress = |completed: usize, path: &Path| {
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
    match scanner::scan_with_progress(&db, scanner::ScanMode::Quick, None, &mut sync_progress) {
        Ok(stats) => {
            if stats.hashes_pending > 0 {
                scanner::schedule_deep_hash_scan(request.root.clone());
            }
            None
        }
        Err(err) => Some(format!("sync source index: {err}")),
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
