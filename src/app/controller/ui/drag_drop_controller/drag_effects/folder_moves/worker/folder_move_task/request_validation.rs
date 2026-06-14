use super::result::error_result;
use super::*;

/// Validate the request, open its source DB, and snapshot entries before mutation.
pub(super) fn prepare_folder_move_transaction(
    request: FolderMoveRequest,
) -> Result<FolderMoveTransaction, FolderMoveResult> {
    let prepared = prepare_folder_move(&request)?;
    let db = SourceDatabase::open(&request.source_root).map_err(|err| {
        error_result(
            &request,
            prepared.new_relative.clone(),
            format!("Failed to open source DB: {err}"),
            false,
        )
    })?;
    let entries = load_folder_entries(&db, &request, &prepared)?;
    Ok(FolderMoveTransaction {
        request,
        prepared,
        db,
        entries,
        moved: Vec::new(),
    })
}

/// Validate the request and derive the old/new filesystem locations.
fn prepare_folder_move(
    request: &FolderMoveRequest,
) -> Result<PreparedFolderMove, FolderMoveResult> {
    if request.folder.as_os_str().is_empty() {
        return Err(error_result(
            request,
            request.target_folder.clone(),
            "Root folder cannot be moved",
            false,
        ));
    }
    if request.target_folder == request.folder {
        return Err(error_result(
            request,
            request.target_folder.clone(),
            "Folder is already there",
            false,
        ));
    }
    if request.target_folder.starts_with(&request.folder) {
        return Err(error_result(
            request,
            request.target_folder.clone(),
            "Cannot move a folder into itself",
            false,
        ));
    }
    let folder_name = match request.folder.file_name() {
        Some(name) => name.to_owned(),
        None => {
            return Err(error_result(
                request,
                request.target_folder.clone(),
                "Folder name unavailable for move",
                false,
            ));
        }
    };
    let new_relative = if request.target_folder.as_os_str().is_empty() {
        PathBuf::from(folder_name)
    } else {
        request.target_folder.join(folder_name)
    };
    let absolute_old = request.source_root.join(&request.folder);
    if !absolute_old.is_dir() {
        return Err(error_result(
            request,
            new_relative,
            format!("Folder not found: {}", request.folder.display()),
            false,
        ));
    }
    if !request.target_folder.as_os_str().is_empty() {
        let destination_dir = request.source_root.join(&request.target_folder);
        if !destination_dir.is_dir() {
            return Err(error_result(
                request,
                new_relative,
                format!("Folder not found: {}", request.target_folder.display()),
                false,
            ));
        }
    }
    let absolute_new = request.source_root.join(&new_relative);
    if absolute_new.exists() {
        let message = format!("Folder already exists: {}", new_relative.display());
        return Err(error_result(request, new_relative, message, false));
    }
    Ok(PreparedFolderMove {
        new_relative,
        absolute_old,
        absolute_new,
    })
}

/// Load source DB rows that need their relative path rewritten after the move.
fn load_folder_entries(
    db: &SourceDatabase,
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
) -> Result<Vec<FolderMoveEntry>, FolderMoveResult> {
    let entries = db.list_files().map_err(|err| {
        error_result(
            request,
            prepared.new_relative.clone(),
            format!("Failed to list folder entries: {err}"),
            false,
        )
    })?;
    let mut folder_entries = Vec::new();
    for entry in entries
        .into_iter()
        .filter(|entry| entry.relative_path.starts_with(&request.folder))
    {
        let collection = db
            .collection_for_path(&entry.relative_path)
            .map_err(|err| {
                error_result(
                    request,
                    prepared.new_relative.clone(),
                    format!(
                        "Failed to read collection for {}: {err}",
                        entry.relative_path.display()
                    ),
                    false,
                )
            })?;
        folder_entries.push(FolderMoveEntry { entry, collection });
    }
    Ok(folder_entries)
}
