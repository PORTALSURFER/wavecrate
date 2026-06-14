use std::path::Path;

use super::{
    super::{
        path_helpers::path_id,
        scan_types::{
            FolderVerifyOutcome, FolderVerifyRequest, FolderVerifyResult, FolderVerifySnapshot,
        },
    },
    file_entry_metadata::file_entry,
    traversal::read_sorted_entries,
};

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
