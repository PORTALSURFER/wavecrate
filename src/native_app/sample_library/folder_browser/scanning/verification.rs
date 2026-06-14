use super::{
    super::{
        path_helpers::path_id,
        scan_types::{
            FolderVerifyOutcome, FolderVerifyRequest, FolderVerifyResult, FolderVerifySnapshot,
        },
    },
    metadata::file_entry_for_source_path,
    traversal::read_sorted_entries,
};
use wavecrate::sample_sources::{SourceDatabase, scanner};

pub(in crate::native_app) fn verify_direct_folder(
    request: FolderVerifyRequest,
) -> FolderVerifyResult {
    reconcile_verified_folder(&request);
    let outcome = match read_direct_folder_snapshot(&request) {
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

fn read_direct_folder_snapshot(request: &FolderVerifyRequest) -> DirectFolderSnapshot {
    let path = &request.folder_path;
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
        .map(|entry| file_entry_for_source_path(entry, &request.source_root))
        .collect::<Vec<_>>();
    DirectFolderSnapshot::Available(FolderVerifySnapshot { child_paths, files })
}

fn reconcile_verified_folder(request: &FolderVerifyRequest) {
    let Ok(relative_folder) = request.folder_path.strip_prefix(&request.source_root) else {
        return;
    };
    let Ok(db) = SourceDatabase::open_for_user_metadata_write(&request.source_root) else {
        return;
    };
    let result = if relative_folder.as_os_str().is_empty() {
        scanner::scan_once(&db)
    } else {
        scanner::sync_paths(&db, &[relative_folder.to_path_buf()])
    };
    match result {
        Ok(stats) if stats.hashes_pending > 0 => {
            scanner::schedule_deep_hash_scan(request.source_root.clone());
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(
                folder = %request.folder_path.display(),
                error = %error,
                "Native folder verification skipped source database reconciliation"
            );
        }
    }
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
