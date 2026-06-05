use std::{fs, path::PathBuf};
use wavecrate::sample_sources::SourceDatabase;

use super::{
    FileRenameEdit, FileRenameView, FolderBrowserState, RenameCommitResult,
    path_helpers::{
        file_rename_draft, file_rename_input_id, resolved_file_rename, valid_file_name,
    },
};

impl FolderBrowserState {
    pub(in crate::gui_app) fn file_rename_view(&self, file_id: &str) -> Option<FileRenameView> {
        self.file_rename_edit
            .as_ref()
            .filter(|edit| edit.file_id == file_id)
            .map(|edit| FileRenameView {
                draft: edit.draft.clone(),
                input_id: edit.input_id,
                selection_start: edit.selection_start,
                selection_end: edit.selection_end,
            })
    }

    pub(super) fn begin_file_rename_selected(&mut self) -> Option<u64> {
        let file_id = self.selected_file.clone()?;
        let (file_id, file_name) = self
            .selected_audio_files()
            .into_iter()
            .find(|file| file.id == file_id)
            .map(|file| (file.id.clone(), file.name.clone()))?;

        let input_id = file_rename_input_id(&file_id);
        let draft = file_rename_draft(&file_name);
        let selection_end = draft.chars().count();
        self.file_rename_edit = Some(FileRenameEdit {
            file_id,
            draft,
            input_id,
            selection_start: 0,
            selection_end,
        });
        Some(input_id)
    }

    pub(super) fn commit_file_rename(&mut self, value: String) -> RenameCommitResult {
        let Some(edit) = self.file_rename_edit.take() else {
            return RenameCommitResult::status("No file rename in progress");
        };
        let old_path = PathBuf::from(&edit.file_id);
        let Some(parent) = old_path.parent() else {
            return RenameCommitResult::status("File rename failed: selected file has no parent");
        };
        let Some(new_name) = resolved_file_rename(&old_path, value.trim()) else {
            return RenameCommitResult::status("File rename failed: use a plain file name");
        };
        if !valid_file_name(&new_name) {
            return RenameCommitResult::status("File rename failed: use a plain file name");
        }
        let new_path = parent.join(&new_name);
        if old_path == new_path {
            return RenameCommitResult::status("File rename unchanged");
        }
        if new_path.exists() {
            return RenameCommitResult::status(format!(
                "File rename failed: {new_name} already exists"
            ));
        }
        if let Err(error) = fs::rename(&old_path, &new_path) {
            return RenameCommitResult::status(format!("File rename failed: {error}"));
        }
        let metadata_remap_result = self.persist_renamed_file_metadata(&old_path, &new_path);
        self.rewrite_renamed_file_path(&old_path, &new_path);
        let status = match metadata_remap_result {
            Ok(()) => format!("Renamed file to {new_name}"),
            Err(error) => {
                format!("Renamed file to {new_name}; metadata update failed: {error}")
            }
        };
        RenameCommitResult::remapped(status, old_path, new_path)
    }

    fn persist_renamed_file_metadata(
        &self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> Result<(), String> {
        let Some((root, old_relative)) = self.source_relative_file_path(old_path) else {
            return Ok(());
        };
        let Some((_, new_relative)) = self.source_relative_file_path(new_path) else {
            return Ok(());
        };
        let db =
            SourceDatabase::open_for_user_metadata_write(&root).map_err(|err| err.to_string())?;
        let mut batch = db.write_batch().map_err(|err| err.to_string())?;
        batch
            .remap_wav_file_path(&old_relative, &new_relative)
            .map_err(|err| err.to_string())?;
        batch
            .remap_analysis_sample_identity(&old_relative, &new_relative)
            .map_err(|err| err.to_string())?;
        batch.commit().map_err(|err| err.to_string())
    }
}
