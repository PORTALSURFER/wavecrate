use std::{fs, path::PathBuf};

use super::{
    FileRenameEdit, FileRenameView, FolderBrowserState,
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

    pub(super) fn commit_file_rename(&mut self, value: String) -> String {
        let Some(edit) = self.file_rename_edit.take() else {
            return String::from("No file rename in progress");
        };
        let old_path = PathBuf::from(&edit.file_id);
        let Some(parent) = old_path.parent() else {
            return String::from("File rename failed: selected file has no parent");
        };
        let Some(new_name) = resolved_file_rename(&old_path, value.trim()) else {
            return String::from("File rename failed: use a plain file name");
        };
        if !valid_file_name(&new_name) {
            return String::from("File rename failed: use a plain file name");
        }
        let new_path = parent.join(&new_name);
        if old_path == new_path {
            return String::from("File rename unchanged");
        }
        if new_path.exists() {
            return format!("File rename failed: {new_name} already exists");
        }
        if let Err(error) = fs::rename(&old_path, &new_path) {
            return format!("File rename failed: {error}");
        }
        self.rewrite_renamed_file_path(&old_path, &new_path);
        format!("Renamed file to {new_name}")
    }
}
