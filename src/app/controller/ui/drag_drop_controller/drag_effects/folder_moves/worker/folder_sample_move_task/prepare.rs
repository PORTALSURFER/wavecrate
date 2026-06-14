use super::*;

/// Validate one folder-sample request, load DB metadata, and stage the file move.
pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::folder_moves::worker) fn prepare_folder_sample_move_transaction<
    'a,
>(
    db: &'a SourceDatabase,
    source_root: &Path,
    request: FolderSampleMoveRequest,
) -> Result<FolderSampleMoveTransaction<'a>, String> {
    let absolute = source_root.join(&request.relative_path);
    if !absolute.is_file() {
        return Err(format!("File missing: {}", request.relative_path.display()));
    }
    if let Some(parent) = request.target_relative.parent() {
        let target_dir = source_root.join(parent);
        if !target_dir.is_dir() {
            return Err(format!("Folder not found: {}", parent.display()));
        }
    }
    let target_absolute = source_root.join(&request.target_relative);
    if target_absolute.exists() {
        return Err(format!(
            "A file already exists at {}",
            request.target_relative.display()
        ));
    }
    let metadata = load_sample_move_metadata(db, &request.relative_path)?;
    let prepared = prepare_staged_move(
        db,
        source_root,
        &request.relative_path,
        source_root,
        &request.target_relative,
        metadata.clone(),
    )?;
    Ok(FolderSampleMoveTransaction {
        request,
        db,
        metadata,
        prepared,
    })
}
