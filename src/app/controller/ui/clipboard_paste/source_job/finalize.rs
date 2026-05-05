use super::*;

/// Move the staged file into place and clear the journal entry on success.
pub(super) fn finalize_source_copy(
    db: &SourceDatabase,
    committed: DbCommittedSourcePaste,
) -> Result<SourcePasteAdded, Vec<String>> {
    if let Err(err) = std::fs::rename(
        &committed.staged.prepared.staged_absolute,
        &committed.staged.prepared.absolute,
    ) {
        return Err(vec![format!("Failed to finalize copy: {err}")]);
    }
    let mut errors = Vec::new();
    cleanup::remove_copy_journal_entry(&mut errors, db, &committed.staged.prepared.op_id);
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(SourcePasteAdded {
        relative_path: committed.staged.prepared.relative,
        file_size: committed.staged.file_size,
        modified_ns: committed.staged.modified_ns,
    })
}
