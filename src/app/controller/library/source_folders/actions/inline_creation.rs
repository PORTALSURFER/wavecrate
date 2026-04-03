use super::ops;
use super::*;
use crate::app::controller::jobs::{FileOpMessage, FileOpResult, FolderCreateResult};
use crate::app::state::{InlineFolderEdit, InlineFolderEditKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};

impl AppController {
    pub(crate) fn start_new_folder(&mut self) {
        if self.current_source().is_none() {
            self.set_status("Add a source before creating folders", StatusTone::Info);
            return;
        }
        self.start_new_folder_at_parent(self.focused_folder_path().unwrap_or_default());
    }

    /// Start inline folder creation under one explicit relative parent path.
    pub(crate) fn start_new_folder_at_parent(&mut self, parent: PathBuf) {
        if self.current_source().is_none() {
            self.set_status("Add a source before creating folders", StatusTone::Info);
            return;
        }
        self.begin_inline_folder_creation(parent);
    }

    /// Start inline folder creation from one folder row in the controller-owned folder list.
    pub(crate) fn start_new_folder_at_folder_row(&mut self, row_index: usize) {
        let Some(row) = self.ui.sources.folders.rows.get(row_index) else {
            self.set_status("Focus a folder to create inside it", StatusTone::Info);
            return;
        };
        self.start_new_folder_at_parent(row.path.clone());
    }

    pub(crate) fn start_new_folder_at_root(&mut self) {
        self.start_new_folder_at_parent(PathBuf::new());
    }

    pub(crate) fn start_folder_rename(&mut self) {
        let Some(target) = self.focused_folder_path() else {
            self.set_status("Focus a folder to rename it", StatusTone::Info);
            return;
        };
        if target.as_os_str().is_empty() {
            self.set_status("Root folder cannot be renamed", StatusTone::Info);
            return;
        }
        let name = target
            .file_name()
            .and_then(|segment| segment.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| target.to_string_lossy().into_owned());
        self.begin_inline_folder_rename(target, name);
    }

    fn begin_inline_folder_creation(&mut self, parent: PathBuf) {
        self.focus_folder_context();
        self.cancel_inline_folder_edit();
        if !self.ui.sources.folders.search_query.trim().is_empty() {
            self.set_folder_search(String::new());
        }
        self.ensure_folder_expanded_for_creation(&parent);
        self.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
            kind: InlineFolderEditKind::Create {
                parent: parent.clone(),
            },
            name: String::new(),
            focus_requested: true,
            select_all_on_focus_requested: false,
        });
        self.focus_folder_parent_row(&parent);
    }

    fn begin_inline_folder_rename(&mut self, target: PathBuf, name: String) {
        self.focus_folder_context();
        self.cancel_inline_folder_edit();
        self.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
            kind: InlineFolderEditKind::Rename {
                target: target.clone(),
            },
            name,
            focus_requested: true,
            select_all_on_focus_requested: true,
        });
        self.focus_folder_by_path(&target);
        if let Some(index) = self.ui.sources.folders.focused {
            self.ui.sources.folders.scroll_to = Some(index);
        }
    }

    fn focus_folder_parent_row(&mut self, parent: &Path) {
        let focus_index = if parent.as_os_str().is_empty() {
            Some(0)
        } else {
            self.ui
                .sources
                .folders
                .rows
                .iter()
                .position(|row| row.path == parent)
        };
        if let Some(index) = focus_index {
            self.ui.sources.folders.focused = Some(index);
            self.ui.sources.folders.scroll_to = Some(index);
        }
    }

    pub(crate) fn cancel_inline_folder_edit(&mut self) {
        self.ui.sources.folders.inline_edit = None;
    }

    pub(crate) fn cancel_new_folder_creation(&mut self) {
        if self.has_pending_new_folder_creation() {
            self.cancel_inline_folder_edit();
        }
    }

    pub(crate) fn cancel_folder_rename(&mut self) {
        if self.has_pending_folder_rename() {
            self.cancel_inline_folder_edit();
        }
    }

    /// Keep the active inline folder-create draft focused in the folder browser.
    pub(crate) fn focus_new_folder_creation_input(&mut self) {
        if !self.has_pending_new_folder_creation() {
            return;
        }
        self.focus_inline_folder_edit_input();
    }

    /// Keep the active inline folder-rename draft focused in the folder browser.
    pub(crate) fn focus_folder_rename_input(&mut self) {
        if !self.has_pending_folder_rename() {
            return;
        }
        self.focus_inline_folder_edit_input();
    }

    pub(crate) fn focus_inline_folder_edit_input(&mut self) {
        let Some(edit) = self.ui.sources.folders.inline_edit.as_mut() else {
            return;
        };
        edit.focus_requested = true;
        self.focus_folder_context();
    }

    pub(crate) fn set_new_folder_creation_input(&mut self, value: String) -> bool {
        if !self.has_pending_new_folder_creation() {
            return false;
        }
        self.set_inline_folder_edit_input(value)
    }

    pub(crate) fn set_folder_rename_input(&mut self, value: String) -> bool {
        if !self.has_pending_folder_rename() {
            return false;
        }
        self.set_inline_folder_edit_input(value)
    }

    pub(crate) fn set_inline_folder_edit_input(&mut self, value: String) -> bool {
        let Some(edit) = self.ui.sources.folders.inline_edit.as_mut() else {
            return false;
        };
        edit.name = value;
        edit.focus_requested = true;
        edit.select_all_on_focus_requested = false;
        true
    }

    pub(crate) fn has_pending_new_folder_creation(&self) -> bool {
        matches!(
            self.ui.sources.folders.inline_edit,
            Some(InlineFolderEdit {
                kind: InlineFolderEditKind::Create { .. },
                ..
            })
        )
    }

    pub(crate) fn has_pending_folder_rename(&self) -> bool {
        matches!(
            self.ui.sources.folders.inline_edit,
            Some(InlineFolderEdit {
                kind: InlineFolderEditKind::Rename { .. },
                ..
            })
        )
    }

    pub(crate) fn apply_pending_new_folder_creation(&mut self) -> bool {
        let Some(edit) = self.ui.sources.folders.inline_edit.clone() else {
            return false;
        };
        let InlineFolderEditKind::Create { parent } = edit.kind else {
            return false;
        };
        match self.create_folder(&parent, &edit.name) {
            Ok(()) => {
                self.cancel_inline_folder_edit();
            }
            Err(err) => {
                self.refresh_folder_browser();
                if let Some(edit) = self.ui.sources.folders.inline_edit.as_mut() {
                    edit.focus_requested = true;
                    edit.select_all_on_focus_requested = false;
                }
                self.set_status(err, StatusTone::Error);
            }
        }
        true
    }

    pub(crate) fn apply_pending_folder_rename(&mut self) -> bool {
        let Some(edit) = self.ui.sources.folders.inline_edit.clone() else {
            return false;
        };
        let InlineFolderEditKind::Rename { target } = edit.kind else {
            return false;
        };
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return true;
        };
        if self.warn_if_retained_delete_path_busy(&source.id, &target, "renaming") {
            if let Some(edit) = self.ui.sources.folders.inline_edit.as_mut() {
                edit.focus_requested = true;
                edit.select_all_on_focus_requested = false;
            }
            return true;
        }
        match self.rename_folder(&target, &edit.name) {
            Ok(()) => {
                self.cancel_inline_folder_edit();
            }
            Err(err) => {
                if let Some(edit) = self.ui.sources.folders.inline_edit.as_mut() {
                    edit.focus_requested = true;
                    edit.select_all_on_focus_requested = false;
                }
                self.set_status(err, StatusTone::Error);
            }
        }
        true
    }

    pub(crate) fn apply_active_inline_folder_edit(&mut self) -> bool {
        if self.has_pending_new_folder_creation() {
            return self.apply_pending_new_folder_creation();
        }
        self.apply_pending_folder_rename()
    }

    fn ensure_folder_expanded_for_creation(&mut self, parent: &Path) {
        if parent.as_os_str().is_empty() {
            return;
        }
        let Some(model) = self.current_folder_model_mut() else {
            return;
        };
        if model.expanded.insert(parent.to_path_buf()) {
            let snapshot = model.clone();
            if let Some(source_id) = self.selected_source_id() {
                self.queue_folder_projection_for_pane(
                    self.active_folder_pane(),
                    source_id,
                    snapshot,
                );
            }
        }
    }

    pub(crate) fn create_folder(&mut self, parent: &Path, name: &str) -> Result<(), String> {
        let folder_name = ops::normalize_folder_name(name)?;
        let source = self
            .current_source()
            .ok_or_else(|| "Select a source first".to_string())?;
        let relative = if parent.as_os_str().is_empty() {
            PathBuf::from(&folder_name)
        } else {
            parent.join(&folder_name)
        };
        let destination = source.root.join(&relative);
        if destination.exists() {
            return Err(format!("Folder already exists: {}", relative.display()));
        }
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("File operation already in progress".to_string());
        }
        if cfg!(test) {
            self.begin_pending_file_mutation(&source.id, [relative.clone()]);
            let result = FolderCreateResult {
                source_id: source.id,
                relative_path: relative,
                result: fs::create_dir_all(&destination)
                    .map_err(|err| format!("Failed to create folder: {err}")),
            };
            self.apply_file_op_result(FileOpResult::FolderCreate(result));
            return Ok(());
        }
        self.begin_pending_file_mutation(&source.id, [relative.clone()]);
        self.set_status(
            format!("Creating folder {}...", relative.display()),
            StatusTone::Busy,
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.runtime.jobs.start_file_ops(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                Err(String::from("Folder creation cancelled"))
            } else {
                fs::create_dir_all(&destination)
                    .map_err(|err| format!("Failed to create folder: {err}"))
            };
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::FolderCreate(
                FolderCreateResult {
                    source_id: source.id,
                    relative_path: relative,
                    result,
                },
            )));
        });
        Ok(())
    }
}
