use radiant::widgets::TextInputMessage;
use std::{fs, path::PathBuf};

use super::{
    FolderBrowserState, FolderEntry, FolderRenameEdit, FolderRenameKind, RenameTargetView,
    path_helpers::{
        folder_label, next_available_folder_name, path_id, rename_input_id, valid_folder_name,
    },
};

impl FolderBrowserState {
    pub(in crate::gui_app) fn rename_active(&self) -> bool {
        self.rename_edit.is_some()
            || self.file_rename_edit.is_some()
            || self.collection_rename_edit.is_some()
    }

    pub(in crate::gui_app) fn selected_rename_target(&self) -> RenameTargetView {
        if let Some(file_id) = self.selected_file.as_deref()
            && let Some(file) = self
                .selected_audio_files()
                .into_iter()
                .find(|file| file.id == file_id)
        {
            return RenameTargetView {
                kind: "file",
                label: file.name.clone(),
                is_source_root: false,
            };
        }
        if let Some(collection) = self.selected_collection
            && let Some(entry) = self
                .collections
                .iter()
                .find(|entry| entry.collection == collection)
        {
            return RenameTargetView {
                kind: "collection",
                label: entry.name.clone(),
                is_source_root: false,
            };
        }
        let Some(folder) = self.selected_folder() else {
            return RenameTargetView {
                kind: "none",
                label: String::new(),
                is_source_root: false,
            };
        };
        RenameTargetView {
            kind: "folder",
            label: folder.name.clone(),
            is_source_root: self.selected_folder_is_source_root(),
        }
    }

    pub(in crate::gui_app) fn begin_rename_selected(&mut self) -> Result<Option<u64>, String> {
        self.discard_pending_created_folder();
        if let Some(input_id) = self.begin_file_rename_selected() {
            return Ok(Some(input_id));
        }
        if let Some(collection) = self.selected_collection
            && let Some(input_id) = self.begin_rename_collection(collection)
        {
            return Ok(Some(input_id));
        }

        let Some(folder) = self.find_folder(&self.selected_folder) else {
            return Ok(None);
        };
        if self.selected_folder_is_source_root() {
            return Err(String::from("Select a subfolder to rename"));
        }
        let folder_id = folder.id.clone();
        let draft = folder.name.clone();
        let input_id = rename_input_id(&folder_id);
        self.file_rename_edit = None;
        self.rename_edit = Some(FolderRenameEdit {
            folder_id,
            draft,
            input_id,
            kind: FolderRenameKind::Rename,
        });
        Ok(Some(input_id))
    }

    pub(in crate::gui_app) fn begin_create_subfolder(&mut self) -> Result<Option<u64>, String> {
        if self.selected_file.is_some() {
            return Err(String::from("Select a folder to add a subfolder"));
        }
        let Some(parent) = self.selected_folder().cloned() else {
            return Ok(None);
        };
        let parent_id = parent.id.clone();
        let parent_path = PathBuf::from(&parent.id);
        if !parent_path.is_dir() {
            return Err(String::from(
                "New folder failed: selected folder is missing",
            ));
        }

        let draft = next_available_folder_name(&parent_path);
        let folder_path = parent_path.join(&draft);
        let folder_id = path_id(&folder_path);
        let input_id = rename_input_id(&folder_id);
        let placeholder = FolderEntry {
            id: folder_id.clone(),
            name: draft.clone(),
            children: Vec::new(),
            files: Vec::new(),
        };
        self.file_rename_edit = None;
        self.discard_pending_created_folder();
        if !self.upsert_child_folder(&parent_id, placeholder) {
            return Err(String::from(
                "New folder failed: selected folder is unavailable",
            ));
        }
        self.expanded_folders.insert(parent_id.clone());
        self.selected_folder = folder_id.clone();
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.rename_edit = Some(FolderRenameEdit {
            folder_id,
            draft,
            input_id,
            kind: FolderRenameKind::Create { parent_id },
        });
        Ok(Some(input_id))
    }

    pub(in crate::gui_app) fn apply_rename_input(
        &mut self,
        message: TextInputMessage,
    ) -> Option<String> {
        match message {
            TextInputMessage::Changed { value } => {
                if let Some(status) =
                    self.apply_collection_rename_input(TextInputMessage::Changed {
                        value: value.clone(),
                    })
                {
                    return Some(status);
                }
                if let Some(edit) = &mut self.file_rename_edit {
                    edit.draft = value;
                } else if let Some(edit) = &mut self.rename_edit {
                    edit.draft = value;
                }
                None
            }
            TextInputMessage::Submitted { value } => {
                if let Some(status) =
                    self.apply_collection_rename_input(TextInputMessage::Submitted {
                        value: value.clone(),
                    })
                {
                    return Some(status);
                }
                if self.file_rename_edit.is_some() {
                    Some(self.commit_file_rename(value))
                } else {
                    Some(self.commit_rename(value))
                }
            }
            TextInputMessage::CompletionRequested { .. } => None,
        }
    }

    pub(in crate::gui_app) fn cancel_rename(&mut self) {
        self.clear_drag();
        self.discard_pending_created_folder();
        self.file_rename_edit = None;
    }

    fn commit_rename(&mut self, value: String) -> String {
        let Some(edit) = self.rename_edit.take() else {
            return String::from("No folder rename in progress");
        };
        match edit.kind.clone() {
            FolderRenameKind::Rename => self.commit_existing_folder_rename(edit, value),
            FolderRenameKind::Create { parent_id } => {
                self.commit_created_subfolder(edit, parent_id, value)
            }
        }
    }

    fn commit_existing_folder_rename(&mut self, edit: FolderRenameEdit, value: String) -> String {
        let new_name = value.trim();
        if !valid_folder_name(new_name) {
            return String::from("Folder rename failed: use a plain folder name");
        }
        let old_path = PathBuf::from(&edit.folder_id);
        let Some(parent) = old_path.parent() else {
            return String::from("Folder rename failed: selected folder has no parent");
        };
        let new_path = parent.join(new_name);
        if old_path == new_path {
            return String::from("Folder rename unchanged");
        }
        if new_path.exists() {
            return format!("Folder rename failed: {new_name} already exists");
        }
        if let Err(error) = fs::rename(&old_path, &new_path) {
            return format!("Folder rename failed: {error}");
        }
        self.rewrite_renamed_folder_paths(&old_path, &new_path);
        format!("Renamed folder to {new_name}")
    }

    fn commit_created_subfolder(
        &mut self,
        edit: FolderRenameEdit,
        parent_id: String,
        value: String,
    ) -> String {
        let new_name = value.trim();
        if !valid_folder_name(new_name) {
            self.remove_pending_created_folder(&edit.folder_id, &parent_id);
            return String::from("New folder failed: use a plain folder name");
        }
        let parent_path = PathBuf::from(&parent_id);
        let new_path = parent_path.join(new_name);
        if new_path.exists() {
            self.remove_pending_created_folder(&edit.folder_id, &parent_id);
            return format!("New folder failed: {new_name} already exists");
        }
        if let Err(error) = fs::create_dir(&new_path) {
            self.remove_pending_created_folder(&edit.folder_id, &parent_id);
            return format!("New folder failed: {error}");
        }

        self.remove_pending_created_folder(&edit.folder_id, &parent_id);
        let new_id = path_id(&new_path);
        self.upsert_child_folder(
            &parent_id,
            FolderEntry {
                id: new_id.clone(),
                name: folder_label(&new_path),
                children: Vec::new(),
                files: Vec::new(),
            },
        );
        self.expanded_folders.insert(parent_id);
        self.selected_folder = new_id;
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.file_view_start = 0;
        format!("Created folder {new_name}")
    }
}
