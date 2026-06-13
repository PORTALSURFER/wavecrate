use radiant::widgets::{TextInputMessage, TextInputMessageKind};
use std::path::PathBuf;

use super::{
    FileRenameEdit, FolderBrowserState, FolderEntry, FolderRenameEdit, FolderRenameKind,
    RenameCommitCompletion, RenameCommitRequest, RenameCommitResult, RenameCommitSuccess,
    RenameInputResult, RenameTargetView,
    path_helpers::{path_id, rename_input_id, valid_folder_name},
};

#[derive(Clone, Debug, Default)]
pub(super) struct BrowserRenameState {
    pub(super) folder: Option<FolderRenameEdit>,
    pub(super) file: Option<FileRenameEdit>,
}

impl FolderBrowserState {
    pub(in crate::native_app) fn rename_active(&self) -> bool {
        self.rename.folder.is_some()
            || self.rename.file.is_some()
            || self.collection_panel.rename_edit.is_some()
    }

    pub(in crate::native_app) fn selected_rename_target(&self) -> RenameTargetView {
        if let Some(file_id) = self.selection.selected_file.as_deref()
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
        if let Some(collection) = self.selection.selected_collection
            && let Some(entry) = self
                .collection_panel
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

    pub(in crate::native_app) fn begin_rename_selected(&mut self) -> Result<Option<u64>, String> {
        self.discard_pending_created_folder();
        if let Some(input_id) = self.begin_file_rename_selected() {
            return Ok(Some(input_id));
        }
        if let Some(collection) = self.selection.selected_collection
            && let Some(input_id) = self.begin_rename_collection(collection)
        {
            return Ok(Some(input_id));
        }

        let Some(folder) = self.find_folder(&self.selection.selected_folder) else {
            return Ok(None);
        };
        if self.selected_folder_is_source_root() {
            return Err(String::from("Select a subfolder to rename"));
        }
        let folder_id = folder.id.clone();
        let draft = folder.name.clone();
        let input_id = rename_input_id(&folder_id);
        self.rename.file = None;
        self.rename.folder = Some(FolderRenameEdit {
            folder_id,
            draft,
            input_id,
            kind: FolderRenameKind::Rename,
        });
        Ok(Some(input_id))
    }

    pub(in crate::native_app) fn begin_create_subfolder(&mut self) -> Result<Option<u64>, String> {
        if self.selection.selected_file.is_some() {
            return Err(String::from("Select a folder to add a subfolder"));
        }
        let Some(parent) = self.selected_folder().cloned() else {
            return Ok(None);
        };
        let parent_id = parent.id.clone();
        let parent_path = PathBuf::from(&parent.id);
        let draft = next_available_child_folder_name(&parent);
        let folder_path = parent_path.join(&draft);
        let folder_id = path_id(&folder_path);
        let input_id = rename_input_id(&folder_id);
        let placeholder = FolderEntry {
            id: folder_id.clone(),
            name: draft.clone(),
            children: Vec::new(),
            files: Vec::new(),
        };
        self.rename.file = None;
        self.discard_pending_created_folder();
        if !self.upsert_child_folder(&parent_id, placeholder) {
            return Err(String::from(
                "New folder failed: selected folder is unavailable",
            ));
        }
        self.tree.expanded_folders.insert(parent_id.clone());
        self.selection.selected_folder = folder_id.clone();
        self.selection.selected_file = None;
        self.selection.selected_file_ids.clear();
        self.selection.selected_file_ids_explicit = false;
        self.rename.folder = Some(FolderRenameEdit {
            folder_id,
            draft,
            input_id,
            kind: FolderRenameKind::Create { parent_id },
        });
        Ok(Some(input_id))
    }

    pub(in crate::native_app) fn apply_rename_input(
        &mut self,
        message: TextInputMessage,
    ) -> Option<RenameInputResult> {
        let parts = message.parts();
        if parts.kind == TextInputMessageKind::CompletionRequested {
            return None;
        }

        let value = parts.value.to_owned();
        if let Some(status) = self.apply_collection_rename_input(&message) {
            return Some(RenameInputResult::Status(RenameCommitResult::status(
                status,
            )));
        }

        if parts.kind == TextInputMessageKind::Submitted {
            if self.rename.file.is_some() {
                Some(self.prepare_file_rename_commit(value))
            } else {
                Some(self.prepare_folder_rename_commit(value))
            }
        } else {
            if let Some(edit) = &mut self.rename.file {
                edit.draft = value;
            } else if let Some(edit) = &mut self.rename.folder {
                edit.draft = value;
            }
            None
        }
    }

    pub(in crate::native_app) fn cancel_rename(&mut self) {
        self.clear_drag();
        self.discard_pending_created_folder();
        self.rename.file = None;
        self.collection_panel.rename_edit = None;
    }

    pub(in crate::native_app) fn apply_rename_commit_completion(
        &mut self,
        completion: RenameCommitCompletion,
    ) -> RenameCommitResult {
        match completion.result {
            Ok(success) => self.apply_rename_commit_success(completion.request, success),
            Err(error) => {
                if let RenameCommitRequest::FolderCreate {
                    parent_id,
                    pending_id,
                    ..
                } = completion.request
                {
                    self.remove_pending_created_folder(&pending_id, &parent_id);
                }
                RenameCommitResult::status(error)
            }
        }
    }

    fn apply_rename_commit_success(
        &mut self,
        request: RenameCommitRequest,
        success: RenameCommitSuccess,
    ) -> RenameCommitResult {
        match (request, success) {
            (
                RenameCommitRequest::FolderRename {
                    old_path,
                    new_path,
                    new_name,
                },
                RenameCommitSuccess::FolderRenamed,
            ) => {
                self.rewrite_renamed_folder_paths(&old_path, &new_path);
                RenameCommitResult::remapped(
                    format!("Renamed folder to {new_name}"),
                    old_path,
                    new_path,
                )
            }
            (
                RenameCommitRequest::FolderCreate {
                    parent_id,
                    pending_id,
                    new_path,
                    new_name,
                },
                RenameCommitSuccess::FolderCreated { folder },
            ) => {
                self.remove_pending_created_folder(&pending_id, &parent_id);
                self.upsert_child_folder(&parent_id, folder);
                self.tree.expanded_folders.insert(parent_id);
                self.selection.selected_folder = path_id(&new_path);
                self.selection.selected_file = None;
                self.selection.selected_file_ids.clear();
                self.selection.selected_file_ids_explicit = false;
                self.reset_file_view();
                RenameCommitResult::status(format!("Created folder {new_name}"))
            }
            (
                RenameCommitRequest::FileRename {
                    old_path,
                    new_path,
                    new_name,
                    ..
                },
                RenameCommitSuccess::FileRenamed {
                    metadata_remap_result,
                },
            ) => {
                self.rewrite_renamed_file_path(&old_path, &new_path);
                let status = match metadata_remap_result {
                    Ok(()) => format!("Renamed file to {new_name}"),
                    Err(error) => {
                        format!("Renamed file to {new_name}; metadata update failed: {error}")
                    }
                };
                RenameCommitResult::remapped(status, old_path, new_path)
            }
            (_, _) => RenameCommitResult::status("Rename failed: invalid completion"),
        }
    }

    fn prepare_folder_rename_commit(&mut self, value: String) -> RenameInputResult {
        let Some(edit) = self.rename.folder.take() else {
            return RenameInputResult::Status(RenameCommitResult::status(
                "No folder rename in progress",
            ));
        };
        let result = match edit.kind.clone() {
            FolderRenameKind::Rename => self.prepare_existing_folder_rename(edit, value),
            FolderRenameKind::Create { parent_id } => {
                self.prepare_created_subfolder(edit, parent_id, value)
            }
        };
        match result {
            Ok(request) => RenameInputResult::Commit(request),
            Err(status) => RenameInputResult::Status(status),
        }
    }

    fn prepare_existing_folder_rename(
        &mut self,
        edit: FolderRenameEdit,
        value: String,
    ) -> Result<RenameCommitRequest, RenameCommitResult> {
        let new_name = value.trim();
        if !valid_folder_name(new_name) {
            return Err(RenameCommitResult::status(
                "Folder rename failed: use a plain folder name",
            ));
        }
        let old_path = PathBuf::from(&edit.folder_id);
        let Some(parent) = old_path.parent() else {
            return Err(RenameCommitResult::status(
                "Folder rename failed: selected folder has no parent",
            ));
        };
        let new_path = parent.join(new_name);
        if old_path == new_path {
            return Err(RenameCommitResult::status("Folder rename unchanged"));
        }
        Ok(RenameCommitRequest::FolderRename {
            old_path,
            new_path,
            new_name: new_name.to_owned(),
        })
    }

    fn prepare_created_subfolder(
        &mut self,
        edit: FolderRenameEdit,
        parent_id: String,
        value: String,
    ) -> Result<RenameCommitRequest, RenameCommitResult> {
        let new_name = value.trim();
        if !valid_folder_name(new_name) {
            self.remove_pending_created_folder(&edit.folder_id, &parent_id);
            return Err(RenameCommitResult::status(
                "New folder failed: use a plain folder name",
            ));
        }
        let parent_path = PathBuf::from(&parent_id);
        let new_path = parent_path.join(new_name);
        Ok(RenameCommitRequest::FolderCreate {
            parent_id,
            pending_id: edit.folder_id,
            new_path,
            new_name: new_name.to_owned(),
        })
    }
}

fn next_available_child_folder_name(parent: &FolderEntry) -> String {
    const BASE_NAME: &str = "New folder";
    let existing = parent
        .children
        .iter()
        .map(|child| child.name.to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    if !existing.contains(&BASE_NAME.to_ascii_lowercase()) {
        return String::from(BASE_NAME);
    }
    (2..)
        .map(|index| format!("{BASE_NAME} {index}"))
        .find(|name| !existing.contains(&name.to_ascii_lowercase()))
        .unwrap_or_else(|| String::from(BASE_NAME))
}
