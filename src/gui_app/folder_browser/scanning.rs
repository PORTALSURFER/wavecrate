use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    FileEntry, FolderEntry,
    path_helpers::{file_label, folder_label, path_id},
    types::{
        FolderScanDiscovery, FolderScanItem, FolderScanProgress, FolderScanRequest,
        FolderScanResult,
    },
};

mod file_entry_metadata;
pub(super) use file_entry_metadata::file_entry;

const MAX_SCAN_DEPTH: usize = 3;
const MAX_CHILD_FOLDERS: usize = 80;

pub(super) fn default_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

pub(super) fn load_root_folder(root: PathBuf) -> FolderEntry {
    load_folder(&root, 0).unwrap_or_else(|| FolderEntry {
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

pub(in crate::gui_app) fn scan_source_with_progress(
    request: FolderScanRequest,
    mut progress: impl FnMut(FolderScanProgress),
    mut discovered: impl FnMut(FolderScanDiscovery),
) -> FolderScanResult {
    let mut scan = ScanProgressCounter {
        completed: 0,
        files: 0,
        folders: 0,
    };
    progress(FolderScanProgress {
        task_id: request.task_id,
        source_id: request.source_id.clone(),
        label: request.label.clone(),
        phase: String::from("Scanning"),
        completed: 0,
        total: 0,
        detail: request.root.display().to_string(),
    });
    let folder = load_folder_with_progress(
        &request.root,
        0,
        &request,
        &mut scan,
        &mut progress,
        &mut discovered,
    )
    .unwrap_or_else(|| placeholder_folder(&request.root));
    FolderScanResult {
        task_id: request.task_id,
        source_id: request.source_id,
        label: request.label,
        folder,
        file_count: scan.files,
        folder_count: scan.folders,
    }
}

fn load_folder(path: &Path, depth: usize) -> Option<FolderEntry> {
    if depth > MAX_SCAN_DEPTH {
        return None;
    }
    let entries = read_sorted_entries(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .take(MAX_CHILD_FOLDERS)
        .filter_map(|entry| load_folder(entry, depth + 1))
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(file_entry)
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

struct ScanProgressCounter {
    completed: usize,
    files: usize,
    folders: usize,
}

fn load_folder_with_progress(
    path: &Path,
    depth: usize,
    request: &FolderScanRequest,
    scan: &mut ScanProgressCounter,
    progress: &mut impl FnMut(FolderScanProgress),
    discovered: &mut impl FnMut(FolderScanDiscovery),
) -> Option<FolderEntry> {
    if depth > MAX_SCAN_DEPTH {
        return None;
    }
    let entries = read_sorted_entries(path);
    let parent_id = path_id(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .take(MAX_CHILD_FOLDERS)
        .filter_map(|entry| {
            scan.completed += 1;
            scan.folders += 1;
            maybe_report_scan_progress(entry, request, scan, progress);
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::Folder(placeholder_folder(entry)),
            });
            let child =
                load_folder_with_progress(entry, depth + 1, request, scan, progress, discovered)?;
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::Folder(child.clone()),
            });
            Some(child)
        })
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(|entry| {
            scan.completed += 1;
            scan.files += 1;
            maybe_report_scan_progress(entry, request, scan, progress);
            let file = file_entry(entry);
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::File(file.clone()),
            });
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

fn maybe_report_scan_progress(
    path: &Path,
    request: &FolderScanRequest,
    scan: &ScanProgressCounter,
    progress: &mut impl FnMut(FolderScanProgress),
) {
    if scan.completed == 1 || scan.completed.is_multiple_of(64) {
        progress(FolderScanProgress {
            task_id: request.task_id,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Scanning"),
            completed: scan.completed,
            total: 0,
            detail: path.display().to_string(),
        });
    }
}

pub(super) fn merge_scan_discovery(root: &mut FolderEntry, event: &FolderScanDiscovery) -> bool {
    let Some(parent) = root.find_mut(&event.parent_id) else {
        return false;
    };
    match &event.item {
        FolderScanItem::Folder(folder) => upsert_folder(&mut parent.children, folder.clone()),
        FolderScanItem::File(file) => upsert_file(&mut parent.files, file.clone()),
    }
}

pub(super) fn upsert_folder(folders: &mut Vec<FolderEntry>, folder: FolderEntry) -> bool {
    match folders.binary_search_by(|candidate| {
        candidate
            .name
            .to_ascii_lowercase()
            .cmp(&folder.name.to_ascii_lowercase())
    }) {
        Ok(index) if folders[index] == folder => false,
        Ok(index) => {
            folders[index] = folder;
            true
        }
        Err(index) => {
            folders.insert(index, folder);
            true
        }
    }
}

pub(super) fn upsert_file(files: &mut Vec<FileEntry>, file: FileEntry) -> bool {
    match files.binary_search_by(|candidate| {
        candidate
            .name
            .to_ascii_lowercase()
            .cmp(&file.name.to_ascii_lowercase())
    }) {
        Ok(index) if files[index] == file => false,
        Ok(index) => {
            files[index] = file;
            true
        }
        Err(index) => {
            files.insert(index, file);
            true
        }
    }
}

fn read_sorted_entries(path: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        file_label(a)
            .to_ascii_lowercase()
            .cmp(&file_label(b).to_ascii_lowercase())
    });
    entries
}
