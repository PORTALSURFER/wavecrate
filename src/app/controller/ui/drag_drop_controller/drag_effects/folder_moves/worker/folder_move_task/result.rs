use super::*;

/// Roll the filesystem rename back after a DB failure and return a failed result.
pub(super) fn rollback_and_error_result(
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
    message: String,
) -> FolderMoveResult {
    let _ = std::fs::rename(&prepared.absolute_new, &prepared.absolute_old);
    error_result(request, prepared.new_relative.clone(), message, false)
}

/// Return the standard cancelled result payload for folder moves.
pub(super) fn cancelled_result(request: &FolderMoveRequest) -> FolderMoveResult {
    FolderMoveResult {
        source_id: request.source_id.clone(),
        old_folder: request.folder.clone(),
        new_folder: request.target_folder.clone(),
        folder_moved: false,
        moved: Vec::new(),
        errors: Vec::new(),
        cancelled: true,
    }
}

/// Return a failed result payload with one error message.
pub(super) fn error_result(
    request: &FolderMoveRequest,
    new_folder: PathBuf,
    message: impl Into<String>,
    folder_moved: bool,
) -> FolderMoveResult {
    FolderMoveResult {
        source_id: request.source_id.clone(),
        old_folder: request.folder.clone(),
        new_folder,
        folder_moved,
        moved: Vec::new(),
        errors: vec![message.into()],
        cancelled: false,
    }
}

/// Return a successful result payload after the move and DB rewrite both complete.
pub(super) fn success_result(
    request: &FolderMoveRequest,
    new_folder: PathBuf,
    moved: Vec<FolderEntryMove>,
) -> FolderMoveResult {
    FolderMoveResult {
        source_id: request.source_id.clone(),
        old_folder: request.folder.clone(),
        new_folder,
        folder_moved: true,
        moved,
        errors: Vec::new(),
        cancelled: false,
    }
}
