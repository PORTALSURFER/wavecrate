use std::path::PathBuf;

use super::{
    FileMetadataRemap, FileRenameEdit, FileRenameView, FolderBrowserState, RenameCommitRequest,
    RenameCommitResult, RenameInputResult,
    path_helpers::{
        file_rename_draft, file_rename_input_id, resolved_file_rename, valid_file_name,
    },
};

impl FolderBrowserState {
    pub(in crate::native_app) fn file_rename_view(&self, file_id: &str) -> Option<FileRenameView> {
        self.rename
            .file
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
        let file_id = self.selection.selected_file.clone()?;
        let (file_id, file_name) = self
            .selected_audio_files()
            .into_iter()
            .find(|file| file.id == file_id)
            .map(|file| (file.id.clone(), file.name.clone()))?;

        let input_id = file_rename_input_id(&file_id);
        let draft = file_rename_draft(&file_name);
        let selection_end = draft.chars().count();
        self.rename.file = Some(FileRenameEdit {
            file_id,
            draft,
            input_id,
            selection_start: 0,
            selection_end,
        });
        Some(input_id)
    }

    pub(super) fn prepare_file_rename_commit(&mut self, value: String) -> RenameInputResult {
        let Some(edit) = self.rename.file.take() else {
            return RenameInputResult::Status(RenameCommitResult::status(
                "No file rename in progress",
            ));
        };
        let old_path = PathBuf::from(&edit.file_id);
        let Some(parent) = old_path.parent() else {
            return RenameInputResult::Status(RenameCommitResult::status(
                "File rename failed: selected file has no parent",
            ));
        };
        let Some(new_name) = resolved_file_rename(&old_path, value.trim()) else {
            return RenameInputResult::Status(RenameCommitResult::status(
                "File rename failed: use a plain file name",
            ));
        };
        if !valid_file_name(&new_name) {
            return RenameInputResult::Status(RenameCommitResult::status(
                "File rename failed: use a plain file name",
            ));
        }
        let new_path = parent.join(&new_name);
        if old_path == new_path {
            return RenameInputResult::Status(RenameCommitResult::status("File rename unchanged"));
        }
        let metadata_remap = self.file_metadata_remap(&old_path, &new_path);
        RenameInputResult::Commit(RenameCommitRequest::FileRename {
            old_path,
            new_path,
            new_name,
            metadata_remap,
        })
    }

    fn file_metadata_remap(
        &self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> Option<FileMetadataRemap> {
        let (root, old_relative) = self.source_relative_file_path(old_path)?;
        let (_, new_relative) = self.source_relative_file_path(new_path)?;
        Some(FileMetadataRemap {
            source_root: root,
            old_relative,
            new_relative,
        })
    }
}
