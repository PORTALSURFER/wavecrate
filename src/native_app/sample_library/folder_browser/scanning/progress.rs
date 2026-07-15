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
    metadata::{SourceMetadataMap, rated_file_entry, source_rating_map_with_rating_decay},
    traversal::{placeholder_folder, read_sorted_entries},
};
use wavecrate::sample_sources::{SourceDatabase, scanner};

/// Publish at most one source-index progress update per bounded file batch.
pub(in crate::native_app) const INDEX_PROGRESS_REPORT_INTERVAL: usize = 128;

pub(in crate::native_app) fn scan_source_with_progress(
    request: FolderScanRequest,
    mut progress: impl FnMut(FolderScanProgress),
    mut discovered: impl FnMut(FolderScanDiscovery),
) -> FolderScanResult {
    let source_root_available = request.root.is_dir();
    let source_db_error = if source_root_available {
        sync_source_database(&request, &mut progress)
    } else {
        None
    };
    let ratings = if source_root_available {
        source_rating_map_with_rating_decay(
            &request.root,
            &request.database_root,
            request.rating_decay_weeks,
        )
    } else {
        SourceMetadataMap::new()
    };
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
    FolderScanResult {
        task_id: request.task_id,
        source_id: request.source_id,
        label: request.label,
        folder,
        missing_collection_snapshot,
        file_count,
        folder_count,
        source_db_error,
        source_root_available,
    }
}

fn sync_source_database(
    request: &FolderScanRequest,
    progress: &mut impl FnMut(FolderScanProgress),
) -> Option<String> {
    let db = match SourceDatabase::open_for_background_job_with_database_root(
        &request.root,
        &request.database_root,
    ) {
        Ok(db) => db,
        Err(err) => return Some(format!("open source index: {err}")),
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
        None,
        &mut sync_progress,
    ) {
        Ok(stats) => stats,
        Err(err) => return Some(format!("sync source index: {err}")),
    };
    let completed = match scanner::complete_deferred_rename_candidates(&db, stats) {
        Ok(completed) => completed,
        Err(err) => return Some(format!("finish deferred rename hashing: {err}")),
    };
    if completed.hashes_pending > 0 {
        scanner::schedule_deep_hash_scan_with_database_root(
            request.root.clone(),
            request.database_root.clone(),
        );
    }
    None
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

    fn record_folder_snapshot_start(&mut self, folder_id: &str) {
        (self.discovered)(FolderScanDiscovery {
            task_id: self.request.task_id,
            source_id: self.request.source_id.clone(),
            parent_id: folder_id.to_string(),
            item: FolderScanItem::ResetFolder,
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
    scan.record_folder_snapshot_start(&parent_id);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .filter_map(|entry| {
            scan.record_folder(entry, &parent_id);
            load_folder_with_progress(entry, scan)
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
