#![allow(missing_docs)]

use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::LayoutOutput,
    layout::Vector2,
    prelude as ui,
    runtime::{PaintFillRect, PaintPrimitive, PaintStrokeRect},
    theme::ThemeTokens,
    widgets::{
        DragHandleMessage, FocusBehavior, PaintBounds, PointerButton, PointerModifiers,
        TextInputMessage, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
        WidgetStyle, WidgetTone,
    },
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use super::GuiMessage;

const MAX_SCAN_DEPTH: usize = 3;
const MAX_CHILD_FOLDERS: usize = 80;
const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 4.0;
const FOLDER_RENAME_INPUT_BASE_ID: u64 = 70_000_000;
const FILE_RENAME_INPUT_BASE_ID: u64 = 80_000_000;
const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;

#[derive(Clone, Debug)]
pub(super) struct FolderBrowserState {
    selected_source: String,
    sources: Vec<SourceEntry>,
    selected_folder: String,
    selected_file: Option<String>,
    selected_file_ids: HashSet<String>,
    expanded_folders: HashSet<String>,
    folders: Vec<FolderEntry>,
    rename_edit: Option<FolderRenameEdit>,
    file_rename_edit: Option<FileRenameEdit>,
    drag: Option<FolderBrowserDrag>,
    drag_pointer: Option<Point>,
    drop_target_folder: Option<String>,
    file_columns: Vec<FileColumn>,
    file_sort: ui::DetailsSort,
    file_column_resize: Option<FileColumnResize>,
    file_view_start: usize,
}

impl FolderBrowserState {
    pub(super) fn load_default() -> Self {
        Self::from_root(default_root_path())
    }

    fn from_root(root: PathBuf) -> Self {
        let sources = vec![SourceEntry::new("assets", "Assets", root)];
        Self::from_sources(sources, String::from("assets"))
    }

    fn from_sources(sources: Vec<SourceEntry>, selected_source: String) -> Self {
        let mut sources = sources;
        let source_index = sources
            .iter()
            .position(|source| source.id == selected_source)
            .or(if sources.is_empty() { None } else { Some(0) })
            .expect("folder browser needs at least one source");
        let root_folder = load_root_folder(sources[source_index].root.clone());
        sources[source_index].root_folder = Some(root_folder.clone());
        let root_id = root_folder.id.clone();
        Self {
            selected_source: sources[source_index].id.clone(),
            sources,
            selected_folder: root_id.clone(),
            selected_file: None,
            selected_file_ids: HashSet::new(),
            expanded_folders: [root_id].into_iter().collect(),
            folders: vec![root_folder],
            rename_edit: None,
            file_rename_edit: None,
            drag: None,
            drag_pointer: None,
            drop_target_folder: None,
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_view_start: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn root_path(&self) -> &Path {
        self.folders
            .first()
            .map(|folder| Path::new(&folder.id))
            .unwrap_or_else(|| Path::new(""))
    }

    #[cfg(test)]
    pub(super) fn source_labels(&self) -> Vec<String> {
        self.sources
            .iter()
            .map(|source| source.label.clone())
            .collect()
    }

    pub(super) fn selected_files(&self) -> &[FileEntry] {
        self.selected_folder()
            .map(|folder| folder.files.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn selected_audio_files(&self) -> Vec<&FileEntry> {
        let mut files = self
            .selected_files()
            .iter()
            .filter(|file| file.is_audio())
            .collect::<Vec<_>>();
        self.sort_files(&mut files);
        files
    }

    pub(super) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.file_columns.iter().collect()
    }

    pub(super) fn file_sort(&self) -> &ui::DetailsSort {
        &self.file_sort
    }

    pub(super) fn selected_file_id(&self) -> Option<&str> {
        self.selected_file.as_deref()
    }

    pub(super) fn selected_file_paths(&self) -> Vec<PathBuf> {
        let selected = if self.selected_file_ids.is_empty() {
            self.selected_file
                .as_deref()
                .map(|id| [id.to_string()].into_iter().collect())
                .unwrap_or_default()
        } else {
            self.selected_file_ids.clone()
        };
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .map(|file| PathBuf::from(&file.id))
            .collect()
    }

    pub(super) fn is_file_selected(&self, file_id: &str) -> bool {
        if self.selected_file_ids.is_empty() {
            return self.selected_file.as_deref() == Some(file_id);
        }
        self.selected_file_ids.contains(file_id)
    }

    pub(super) fn selected_audio_file_index(&self) -> Option<usize> {
        let selected = self.selected_file.as_deref()?;
        self.selected_audio_files()
            .iter()
            .position(|file| file.id == selected)
    }

    #[cfg(test)]
    pub(super) fn file_view_start(&self) -> usize {
        self.file_view_start
    }

    pub(super) fn set_file_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.selected_audio_files().len();
        if total_items == 0 {
            self.file_view_start = 0;
            return;
        }
        self.file_view_start = ((offset_y.max(0.0) / row_height.max(1.0)).floor() as usize)
            .min(total_items.saturating_sub(1));
    }

    pub(super) fn follow_selected_file_view(
        &mut self,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let total_items = self.selected_audio_files().len();
        if total_items == 0 || viewport_rows == 0 {
            self.file_view_start = 0;
            return ui::VirtualListWindow {
                total_items,
                ..Default::default()
            };
        }
        let viewport_rows = viewport_rows.min(total_items).max(1);
        let guard_rows = guard_rows.min(viewport_rows.saturating_sub(1) / 2);
        let overscan_rows = overscan_rows.min(total_items.saturating_sub(viewport_rows));
        let mut viewport_start = self.file_view_start.min(total_items.saturating_sub(1));
        if let Some(focused_index) = self.selected_audio_file_index() {
            let lower_guard = viewport_start.saturating_add(guard_rows);
            let upper_guard = viewport_start
                .saturating_add(viewport_rows.saturating_sub(1))
                .saturating_sub(guard_rows.saturating_add(1));
            if focused_index <= lower_guard {
                viewport_start = focused_index.saturating_sub(guard_rows);
            } else if focused_index >= upper_guard {
                viewport_start = focused_index.saturating_sub(
                    viewport_rows
                        .saturating_sub(1)
                        .saturating_sub(guard_rows.saturating_add(1)),
                );
            }
        }
        self.file_view_start = viewport_start.min(total_items.saturating_sub(1));
        let viewport_end = self
            .file_view_start
            .saturating_add(viewport_rows)
            .min(total_items);
        let window_start = self.file_view_start.saturating_sub(overscan_rows);
        let window_end = viewport_end.saturating_add(overscan_rows).min(total_items);
        ui::VirtualListWindow {
            total_items,
            viewport_start: self.file_view_start,
            viewport_end,
            window_start,
            window_end,
        }
    }

    pub(super) fn rename_active(&self) -> bool {
        self.rename_edit.is_some() || self.file_rename_edit.is_some()
    }

    pub(super) fn selected_rename_target(&self) -> RenameTargetView {
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

    pub(super) fn file_rename_view(&self, file_id: &str) -> Option<FileRenameView> {
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

    pub(super) fn scan_is_active(&self, source_id: &str, task_id: u64) -> bool {
        self.sources
            .iter()
            .any(|source| source.id == source_id && source.loading_task == Some(task_id))
    }

    pub(super) fn apply_message(&mut self, message: FolderBrowserMessage) {
        match message {
            FolderBrowserMessage::AddSource
            | FolderBrowserMessage::SelectSource(_)
            | FolderBrowserMessage::BeginRenameSelected
            | FolderBrowserMessage::BeginCreateSubfolder
            | FolderBrowserMessage::RenameInput(_)
            | FolderBrowserMessage::DropOnFolder(_) => {}
            FolderBrowserMessage::ClearDropTarget => {
                self.drop_target_folder = None;
            }
            FolderBrowserMessage::HoverDropTarget(id) => {
                self.hover_drop_target_folder(&id);
            }
            FolderBrowserMessage::ActivateFolder(id) => {
                self.cancel_rename();
                self.activate_folder(id);
            }
            FolderBrowserMessage::DragFolder(id, message) => {
                self.apply_folder_drag(id, message);
            }
            FolderBrowserMessage::SortFileColumn(column_id) => {
                self.sort_file_column(column_id);
            }
            FolderBrowserMessage::ResizeFileColumn(column_id, message) => {
                self.resize_file_column(column_id, message);
            }
        }
    }

    pub(super) fn begin_rename_selected(&mut self) -> Result<Option<u64>, String> {
        self.discard_pending_created_folder();
        if let Some(file_id) = self.selected_file.clone() {
            if let Some((file_id, file_name)) = self
                .selected_audio_files()
                .into_iter()
                .find(|file| file.id == file_id)
                .map(|file| (file.id.clone(), file.name.clone()))
            {
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
                return Ok(Some(input_id));
            }
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

    pub(super) fn begin_create_subfolder(&mut self) -> Result<Option<u64>, String> {
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

    pub(super) fn selected_delete_target(&self) -> Result<FolderDeleteTargetView, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before deleting a folder"));
        }
        if self.selected_file.is_some() {
            return Err(String::from("Select a folder to delete"));
        }
        let Some(folder) = self.selected_folder() else {
            return Err(String::from("Select a folder to delete"));
        };
        if self.selected_folder_is_source_root() {
            return Err(String::from("Root folder cannot be deleted"));
        }
        Ok(FolderDeleteTargetView {
            path: PathBuf::from(&folder.id),
            name: folder.name.clone(),
        })
    }

    pub(super) fn selected_file_delete_target(&self) -> Result<FileDeleteTargetView, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before deleting a file"));
        }
        if self.selected_file.is_none() {
            return Err(String::from("Select a file to delete"));
        }
        let paths = self.selected_file_paths();
        if paths.is_empty() {
            return Err(String::from("Select a file to delete"));
        }
        let names = paths
            .iter()
            .map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            })
            .collect();
        Ok(FileDeleteTargetView { paths, names })
    }

    pub(super) fn delete_selected_folder(&mut self) -> Result<String, String> {
        let target = self.selected_delete_target()?;
        if !target.path.is_dir() {
            return Err(format!("Folder delete failed: {} is missing", target.name));
        }
        Err(String::from(
            "Trash workflow is not available in the default GUI yet; no folder was deleted",
        ))
    }

    pub(super) fn delete_selected_files(&mut self) -> Result<String, String> {
        let target = self.selected_file_delete_target()?;
        for path in &target.paths {
            if !path.is_file() {
                return Err(format!(
                    "File delete failed: {} is missing",
                    path.file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string())
                ));
            }
        }
        Err(String::from(
            "Trash workflow is not available in the default GUI yet; no files were deleted",
        ))
    }

    pub(super) fn begin_file_drag(&mut self, file_id: String, position: Point) {
        if self.rename_active() || !self.selected_files().iter().any(|file| file.id == file_id) {
            return;
        }
        let file_ids = if self.selected_file_ids.contains(&file_id) {
            let mut ids = self.selected_file_ids.iter().cloned().collect::<Vec<_>>();
            ids.sort();
            ids
        } else {
            vec![file_id]
        };
        self.drag = Some(FolderBrowserDrag::Files { file_ids });
        self.drag_pointer = Some(position);
        self.drop_target_folder = None;
    }

    pub(super) fn update_drag_pointer(&mut self, position: Point) {
        if self.drag.is_some() {
            self.drag_pointer = Some(position);
        }
    }

    pub(super) fn drag_preview(&self) -> Option<FolderDragPreview> {
        let drag = self.drag.as_ref()?;
        let pointer = self.drag_pointer?;
        Some(FolderDragPreview {
            label: self.drag_preview_label(drag)?,
            pointer,
        })
    }

    pub(super) fn external_drag_request(&self) -> Option<ui::ExternalDragRequest> {
        let drag = self.drag.as_ref()?;
        let label = self.drag_preview_label(drag)?;
        let paths = match drag {
            FolderBrowserDrag::Folder { folder_id } => vec![PathBuf::from(folder_id)],
            FolderBrowserDrag::Files { file_ids } => file_ids.iter().map(PathBuf::from).collect(),
        };
        Some(ui::ExternalDragRequest::files(paths, label))
    }

    pub(super) fn clear_drag(&mut self) {
        self.drag = None;
        self.drag_pointer = None;
        self.drop_target_folder = None;
    }

    pub(super) fn drop_drag_on_folder(
        &mut self,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        let Some(drag) = self.drag.clone() else {
            return Ok(FolderDropResult::default());
        };
        if !self.can_drop_drag_on_folder(target_folder_id) {
            self.clear_drag();
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Drop target unchanged")),
            });
        }
        self.drop_target_folder = None;
        let result = match drag {
            FolderBrowserDrag::Folder { folder_id } => {
                self.move_folder_to_folder(&folder_id, target_folder_id)?
            }
            FolderBrowserDrag::Files { file_ids } => {
                self.move_files_to_folder(&file_ids, target_folder_id)?
            }
        };
        self.clear_drag();
        Ok(result)
    }

    pub(super) fn apply_rename_input(&mut self, message: TextInputMessage) -> Option<String> {
        match message {
            TextInputMessage::Changed { value } => {
                if let Some(edit) = &mut self.file_rename_edit {
                    edit.draft = value;
                } else if let Some(edit) = &mut self.rename_edit {
                    edit.draft = value;
                }
                None
            }
            TextInputMessage::Submitted { value } => {
                if self.file_rename_edit.is_some() {
                    Some(self.commit_file_rename(value))
                } else {
                    Some(self.commit_rename(value))
                }
            }
        }
    }

    pub(super) fn navigate_vertical(&mut self, delta: i32, extend: bool) -> Option<String> {
        if delta == 0 || self.rename_active() {
            return None;
        }
        if self.selected_file.is_some() {
            return self.navigate_selected_file(delta, extend);
        }
        self.navigate_selected_folder(delta);
        None
    }

    pub(super) fn collapse_selected_folder(&mut self) -> bool {
        if self.rename_active() {
            return false;
        }
        if self.folder_has_children(&self.selected_folder) {
            self.expanded_folders.remove(&self.selected_folder)
        } else {
            false
        }
    }

    pub(super) fn expand_selected_folder(&mut self) -> bool {
        if self.rename_active() {
            return false;
        }
        if self.folder_has_children(&self.selected_folder) {
            self.expanded_folders.insert(self.selected_folder.clone())
        } else {
            false
        }
    }

    pub(super) fn begin_add_source_path(
        &mut self,
        root: PathBuf,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        if let Some(index) = self.sources.iter().position(|source| source.root == root) {
            let id = self.sources[index].id.clone();
            return self.begin_select_source(id, task_id);
        }
        let id = path_id(&root);
        let label = folder_label(&root);
        let mut source = SourceEntry::new(id.clone(), label.clone(), root.clone());
        source.loading_task = Some(task_id);
        self.sources.push(source);
        self.select_pending_source(id.clone(), placeholder_folder(&root));
        Some(FolderScanRequest {
            task_id,
            source_id: id,
            label,
            root,
        })
    }

    pub(super) fn begin_select_source(
        &mut self,
        id: String,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        let index = self.sources.iter().position(|source| source.id == id)?;
        if self.selected_source == id && self.sources[index].root_folder.is_some() {
            return None;
        }
        if let Some(root_folder) = self.sources[index].root_folder.clone() {
            self.select_loaded_source(id, root_folder);
            return None;
        }
        if self.sources[index].loading_task.is_some() {
            let root = self.sources[index].root.clone();
            self.select_pending_source(id, placeholder_folder(&root));
            return None;
        }
        self.sources[index].loading_task = Some(task_id);
        let source = self.sources[index].clone();
        self.select_pending_source(source.id.clone(), placeholder_folder(&source.root));
        Some(FolderScanRequest {
            task_id,
            source_id: source.id,
            label: source.label,
            root: source.root,
        })
    }

    pub(super) fn apply_scan_finished(&mut self, result: FolderScanResult) -> bool {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == result.source_id)
        else {
            return false;
        };
        if source.loading_task != Some(result.task_id) {
            return false;
        }
        let source_id = source.id.clone();
        let should_select = self.selected_source == source_id;
        source.loading_task = None;
        source.root_folder = Some(result.folder.clone());
        if should_select {
            self.select_loaded_source(source_id, result.folder);
        }
        true
    }

    #[cfg(test)]
    pub(super) fn apply_scan_discovered(&mut self, event: FolderScanDiscovery) -> bool {
        self.apply_scan_discovered_batch(FolderScanDiscoveryBatch {
            task_id: event.task_id,
            source_id: event.source_id.clone(),
            events: vec![event],
        })
    }

    pub(super) fn apply_scan_discovered_batch(&mut self, batch: FolderScanDiscoveryBatch) -> bool {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == batch.source_id)
        else {
            return false;
        };
        if source.loading_task != Some(batch.task_id) {
            return false;
        }

        let root_folder = source
            .root_folder
            .get_or_insert_with(|| placeholder_folder(&source.root));
        let mut changed = false;
        for event in &batch.events {
            changed |= merge_scan_discovery(root_folder, event);
        }
        if changed && self.selected_source == batch.source_id {
            self.folders = vec![root_folder.clone()];
        }
        changed
    }

    fn select_pending_source(&mut self, id: String, folder: FolderEntry) {
        self.cancel_rename();
        let root_id = folder.id.clone();
        self.selected_source = id;
        self.selected_folder = root_id.clone();
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.file_view_start = 0;
        self.expanded_folders.clear();
        self.expanded_folders.insert(root_id);
        self.folders = vec![folder];
    }

    fn select_loaded_source(&mut self, id: String, root_folder: FolderEntry) {
        self.cancel_rename();
        let root_id = root_folder.id.clone();
        self.selected_source = id;
        self.selected_folder = root_id.clone();
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.file_view_start = 0;
        self.expanded_folders.clear();
        self.expanded_folders.insert(root_id);
        self.folders = vec![root_folder];
    }

    fn selected_folder(&self) -> Option<&FolderEntry> {
        self.find_folder(&self.selected_folder)
            .or_else(|| self.folders.first())
    }

    fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    fn is_expanded(&self, id: &str) -> bool {
        self.expanded_folders.contains(id)
    }

    fn activate_folder(&mut self, id: String) {
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selected_folder == id {
            self.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    fn select_folder(&mut self, id: String) {
        self.cancel_rename();
        self.selected_folder = id;
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.file_view_start = 0;
    }

    fn navigate_selected_folder(&mut self, delta: i32) -> bool {
        let folders = self.visible_folders();
        let Some(current_index) = folders
            .iter()
            .position(|folder| folder.id == self.selected_folder)
        else {
            return false;
        };
        let target_index = offset_index(current_index, delta, folders.len());
        if target_index == current_index {
            return false;
        }
        self.select_folder(folders[target_index].id.clone());
        true
    }

    fn navigate_selected_file(&mut self, delta: i32, extend: bool) -> Option<String> {
        let files = self.selected_audio_files();
        let current = self.selected_file.as_deref()?;
        let current_index = files.iter().position(|file| file.id == current)?;
        let target_index = offset_index(current_index, delta, files.len());
        if target_index == current_index {
            return None;
        }
        let target = files[target_index].id.clone();
        if extend {
            self.selected_file_ids.insert(current.to_string());
            self.selected_file_ids.insert(target.clone());
        } else {
            self.selected_file_ids.clear();
            self.selected_file_ids.insert(target.clone());
        }
        self.selected_file = Some(target.clone());
        Some(target)
    }

    fn sort_file_column(&mut self, column_id: String) {
        if self.file_sort.column_id == column_id {
            self.file_sort.direction = self.file_sort.direction.toggled();
        } else {
            self.file_sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    fn resize_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        match message {
            DragHandleMessage::Started { position } => {
                if let Some(column) = self
                    .file_columns
                    .iter()
                    .find(|column| column.id == column_id)
                {
                    self.file_column_resize = Some(FileColumnResize {
                        column_id,
                        start_x: position.x,
                        start_width: column.width,
                    });
                }
            }
            DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
                let Some(resize) = self.file_column_resize.clone() else {
                    return;
                };
                if let Some(column) = self
                    .file_columns
                    .iter_mut()
                    .find(|column| column.id == resize.column_id)
                {
                    column.width = (resize.start_width + position.x - resize.start_x)
                        .clamp(MIN_FILE_COLUMN_WIDTH, MAX_FILE_COLUMN_WIDTH);
                }
                if matches!(message, DragHandleMessage::Ended { .. }) {
                    self.file_column_resize = None;
                }
            }
        }
    }

    fn apply_folder_drag(&mut self, folder_id: String, message: DragHandleMessage) {
        if self.rename_active() {
            return;
        }
        match message {
            DragHandleMessage::Started { position } => {
                if self.selected_folder_is_source_root_id(&folder_id) {
                    return;
                }
                if self.find_folder(&folder_id).is_some() {
                    self.drag = Some(FolderBrowserDrag::Folder { folder_id });
                    self.drag_pointer = Some(position);
                    self.drop_target_folder = None;
                }
            }
            DragHandleMessage::Moved { position } => {
                self.update_drag_pointer(position);
            }
            DragHandleMessage::Ended { .. } => {
                self.clear_drag();
            }
        }
    }

    fn hover_drop_target_folder(&mut self, folder_id: &str) {
        if self.can_drop_drag_on_folder(folder_id) {
            self.drop_target_folder = Some(folder_id.to_owned());
        } else {
            self.drop_target_folder = None;
        }
    }

    fn drag_preview_label(&self, drag: &FolderBrowserDrag) -> Option<String> {
        match drag {
            FolderBrowserDrag::Folder { folder_id } => self
                .find_folder(folder_id)
                .map(|folder| folder.name.clone()),
            FolderBrowserDrag::Files { file_ids } => match file_ids.as_slice() {
                [] => None,
                [file_id] => Some(file_label(Path::new(file_id))),
                files => Some(format!("{} files", files.len())),
            },
        }
    }

    fn can_drop_drag_on_folder(&self, target_folder_id: &str) -> bool {
        let Some(target) = self.find_folder(target_folder_id) else {
            return false;
        };
        let target_path = Path::new(&target.id);
        match &self.drag {
            Some(FolderBrowserDrag::Folder { folder_id }) => {
                let Some(source) = self.find_folder(folder_id) else {
                    return false;
                };
                let source_path = Path::new(&source.id);
                !self.selected_folder_is_source_root_id(folder_id)
                    && source.id != target.id
                    && !target_path.starts_with(source_path)
            }
            Some(FolderBrowserDrag::Files { file_ids }) => file_ids.iter().any(|id| {
                let path = Path::new(id);
                path.is_file() && path.parent() != Some(target_path)
            }),
            None => false,
        }
    }

    fn move_folder_to_folder(
        &mut self,
        folder_id: &str,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving a folder"));
        }
        if self.selected_folder_is_source_root_id(folder_id) {
            return Err(String::from("Root folder cannot be moved"));
        }
        let source_folder = self
            .find_folder(folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: source folder is missing"))?;
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: target folder is missing"))?;
        let old_path = PathBuf::from(&source_folder.id);
        let target_path = PathBuf::from(&target_folder.id);
        if target_path.starts_with(&old_path) {
            return Err(String::from(
                "Folder move failed: cannot move a folder into itself",
            ));
        }
        let Some(folder_name) = old_path.file_name() else {
            return Err(String::from(
                "Folder move failed: source folder has no name",
            ));
        };
        let new_path = target_path.join(folder_name);
        if old_path == new_path {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Folder move unchanged")),
            });
        }
        if new_path.exists() {
            return Err(format!(
                "Folder move failed: {} already exists",
                folder_name.to_string_lossy()
            ));
        }
        fs::rename(&old_path, &new_path).map_err(|error| format!("Folder move failed: {error}"))?;
        if let Err(error) = self.relocate_moved_folder(&old_path, &new_path, &target_path) {
            let _ = fs::rename(&new_path, &old_path);
            return Err(error);
        }
        Ok(FolderDropResult {
            moved_paths: vec![(old_path, new_path.clone())],
            status: Some(format!(
                "Moved folder {}",
                new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| new_path.display().to_string())
            )),
        })
    }

    fn move_files_to_folder(
        &mut self,
        file_ids: &[String],
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving files"));
        }
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("File move failed: target folder is missing"))?;
        let target_path = PathBuf::from(&target_folder.id);
        if !target_path.is_dir() {
            return Err(String::from("File move failed: target folder is missing"));
        }
        let mut moves = Vec::new();
        let mut seen = HashSet::new();
        for id in file_ids {
            if !seen.insert(id.clone()) {
                continue;
            }
            let old_path = PathBuf::from(id);
            if !old_path.is_file() {
                return Err(format!(
                    "File move failed: {} is missing",
                    old_path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| old_path.display().to_string())
                ));
            }
            if old_path.parent() == Some(target_path.as_path()) {
                continue;
            }
            let Some(file_name) = old_path.file_name() else {
                return Err(String::from("File move failed: source file has no name"));
            };
            let new_path = target_path.join(file_name);
            if new_path.exists() {
                return Err(format!(
                    "File move failed: {} already exists",
                    file_name.to_string_lossy()
                ));
            }
            moves.push((old_path, new_path));
        }
        if moves.is_empty() {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("File move unchanged")),
            });
        }
        let mut completed = Vec::new();
        for (old_path, new_path) in &moves {
            if let Err(error) = fs::rename(old_path, new_path) {
                for (moved_old, moved_new) in completed.iter().rev() {
                    let _ = fs::rename(moved_new, moved_old);
                }
                return Err(format!("File move failed: {error}"));
            }
            completed.push((old_path.clone(), new_path.clone()));
        }
        if let Err(error) = self.relocate_moved_files(&completed, &target_path) {
            for (moved_old, moved_new) in completed.iter().rev() {
                let _ = fs::rename(moved_new, moved_old);
            }
            return Err(error);
        }
        Ok(FolderDropResult {
            moved_paths: completed.clone(),
            status: Some(format!(
                "Moved {} file{}",
                completed.len(),
                plural(completed.len())
            )),
        })
    }

    fn sort_files<'a>(&self, files: &mut Vec<&'a FileEntry>) {
        files.sort_by(|a, b| {
            let ordering = match self.file_sort.column_id.as_str() {
                "extension" => a
                    .extension
                    .to_ascii_lowercase()
                    .cmp(&b.extension.to_ascii_lowercase())
                    .then_with(|| {
                        a.name
                            .to_ascii_lowercase()
                            .cmp(&b.name.to_ascii_lowercase())
                    }),
                "size" => a.size_bytes.cmp(&b.size_bytes).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "modified" => a.modified_rank.cmp(&b.modified_rank).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "kind" => a.kind.cmp(&b.kind).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "path" => a.id.cmp(&b.id),
                _ => a
                    .name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase()),
            };
            match self.file_sort.direction {
                ui::SortDirection::Ascending => ordering,
                ui::SortDirection::Descending => ordering.reverse(),
            }
        });
    }

    fn selected_folder_is_source_root(&self) -> bool {
        self.sources.iter().any(|source| {
            source.id == self.selected_source && path_id(&source.root) == self.selected_folder
        })
    }

    fn selected_folder_is_source_root_id(&self, folder_id: &str) -> bool {
        self.sources
            .iter()
            .any(|source| source.id == self.selected_source && path_id(&source.root) == folder_id)
    }

    fn cancel_rename(&mut self) {
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

    fn commit_file_rename(&mut self, value: String) -> String {
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

    fn rewrite_renamed_folder_paths(&mut self, old_path: &Path, new_path: &Path) {
        let old_id = path_id(old_path);
        let new_id = path_id(new_path);
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return;
        };
        if let Some(root_folder) = &mut source.root_folder {
            root_folder.rewrite_path_prefix(old_path, new_path);
            self.folders = vec![root_folder.clone()];
        }
        self.selected_folder = rewrite_path_id(&self.selected_folder, old_path, new_path);
        if self.selected_folder == old_id {
            self.selected_folder = new_id;
        }
        self.selected_file = self
            .selected_file
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selected_file_ids = self
            .selected_file_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.expanded_folders = self
            .expanded_folders
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
    }

    fn rewrite_renamed_file_path(&mut self, old_path: &Path, new_path: &Path) {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return;
        };
        if let Some(root_folder) = &mut source.root_folder {
            root_folder.rewrite_file_path(old_path, new_path);
            self.folders = vec![root_folder.clone()];
        }
        let new_id = path_id(new_path);
        self.selected_file = Some(new_id);
        self.selected_file_ids.clear();
        self.selected_file_ids.insert(path_id(new_path));
    }

    fn relocate_moved_folder(
        &mut self,
        old_path: &Path,
        new_path: &Path,
        target_parent: &Path,
    ) -> Result<(), String> {
        let old_id = path_id(old_path);
        let target_parent_id = path_id(target_parent);
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return Err(String::from(
                "Folder move failed: selected source is unavailable",
            ));
        };
        let Some(root_folder) = &mut source.root_folder else {
            return Err(String::from(
                "Folder move failed: source tree is unavailable",
            ));
        };
        let Some(mut moved_folder) = root_folder.take_child_by_id(&old_id) else {
            return Err(String::from(
                "Folder move failed: source folder is unavailable",
            ));
        };
        moved_folder.rewrite_path_prefix(old_path, new_path);
        let Some(target_folder) = root_folder.find_mut(&target_parent_id) else {
            return Err(String::from(
                "Folder move failed: target folder is unavailable",
            ));
        };
        upsert_folder(&mut target_folder.children, moved_folder);
        self.folders = vec![root_folder.clone()];

        self.selected_folder = rewrite_path_id(&self.selected_folder, old_path, new_path);
        self.selected_file = self
            .selected_file
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selected_file_ids = self
            .selected_file_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.expanded_folders = self
            .expanded_folders
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.expanded_folders.insert(target_parent_id);
        Ok(())
    }

    fn relocate_moved_files(
        &mut self,
        moves: &[(PathBuf, PathBuf)],
        target_parent: &Path,
    ) -> Result<(), String> {
        let old_ids = moves
            .iter()
            .map(|(old_path, _)| path_id(old_path))
            .collect::<HashSet<_>>();
        let target_parent_id = path_id(target_parent);
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return Err(String::from(
                "File move failed: selected source is unavailable",
            ));
        };
        let Some(root_folder) = &mut source.root_folder else {
            return Err(String::from("File move failed: source tree is unavailable"));
        };
        root_folder.remove_files_by_ids(&old_ids);
        let Some(target_folder) = root_folder.find_mut(&target_parent_id) else {
            return Err(String::from(
                "File move failed: target folder is unavailable",
            ));
        };
        for (_, new_path) in moves {
            upsert_file(&mut target_folder.files, file_entry(new_path));
        }
        self.folders = vec![root_folder.clone()];
        let moved_ids = moves
            .iter()
            .map(|(_, new_path)| path_id(new_path))
            .collect::<Vec<_>>();
        let rewrite_file_id = |id: &str| {
            moves
                .iter()
                .find(|(old_path, _)| path_id(old_path) == id)
                .map(|(_, new_path)| path_id(new_path))
                .unwrap_or_else(|| id.to_string())
        };
        let selected_file_was_moved = self
            .selected_file
            .as_ref()
            .is_some_and(|id| old_ids.contains(id));
        self.selected_file = if selected_file_was_moved {
            self.selected_file.take().map(|id| rewrite_file_id(&id))
        } else {
            moved_ids.first().cloned()
        };
        self.selected_file_ids = if self.selected_file_ids.iter().any(|id| old_ids.contains(id)) {
            self.selected_file_ids
                .iter()
                .map(|id| rewrite_file_id(id))
                .collect()
        } else {
            moved_ids.iter().cloned().collect()
        };
        self.selected_folder = target_parent_id.clone();
        self.file_view_start = 0;
        self.expanded_folders.insert(target_parent_id);
        Ok(())
    }

    fn discard_pending_created_folder(&mut self) {
        let Some(edit) = self.rename_edit.take() else {
            return;
        };
        if let FolderRenameKind::Create { parent_id } = edit.kind {
            self.remove_pending_created_folder(&edit.folder_id, &parent_id);
        }
    }

    fn remove_pending_created_folder(&mut self, folder_id: &str, parent_id: &str) {
        self.remove_folder_by_id(folder_id);
        if self.selected_folder == folder_id {
            self.selected_folder = if self.find_folder(parent_id).is_some() {
                parent_id.to_string()
            } else {
                self.folders
                    .first()
                    .map(|folder| folder.id.clone())
                    .unwrap_or_default()
            };
        }
        self.expanded_folders.remove(folder_id);
    }

    fn remove_folder_by_id(&mut self, folder_id: &str) -> bool {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return false;
        };
        let Some(root_folder) = &mut source.root_folder else {
            return false;
        };
        let changed = root_folder.remove_child_by_id(folder_id);
        if changed {
            self.folders = vec![root_folder.clone()];
        }
        changed
    }

    fn upsert_child_folder(&mut self, parent_id: &str, folder: FolderEntry) -> bool {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return false;
        };
        let Some(root_folder) = &mut source.root_folder else {
            return false;
        };
        let Some(parent) = root_folder.find_mut(parent_id) else {
            return false;
        };
        let changed = upsert_folder(&mut parent.children, folder);
        if changed {
            self.folders = vec![root_folder.clone()];
        }
        changed
    }

    pub(super) fn select_file(&mut self, id: String) {
        if self.selected_files().iter().any(|file| file.id == id) {
            self.cancel_rename();
            self.selected_file = Some(id.clone());
            self.selected_file_ids.clear();
            self.selected_file_ids.insert(id);
        }
    }

    pub(super) fn select_file_with_modifiers(&mut self, id: String, modifiers: PointerModifiers) {
        if self.rename_active() || !self.selected_files().iter().any(|file| file.id == id) {
            return;
        }
        self.cancel_rename();

        if modifiers.shift {
            self.select_file_range_to(id, modifiers.command);
            return;
        }

        if modifiers.command {
            self.toggle_file_selection(id);
            return;
        }

        self.selected_file = Some(id.clone());
        self.selected_file_ids.clear();
        self.selected_file_ids.insert(id);
    }

    pub(super) fn focus_file_preserving_selection(&mut self, id: String) {
        if self.selected_file_ids.contains(&id)
            && self.selected_files().iter().any(|file| file.id == id)
        {
            self.selected_file = Some(id);
        } else {
            self.select_file(id);
        }
    }

    pub(super) fn select_all_audio_files(&mut self) -> usize {
        if self.rename_active() {
            return self.selected_file_ids.len();
        }
        let ids = self
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        self.selected_file_ids = ids.iter().cloned().collect();
        if self.selected_file.is_none() {
            self.selected_file = ids.first().cloned();
        }
        self.selected_file_ids.len()
    }

    fn select_file_range_to(&mut self, id: String, add_to_existing: bool) {
        let files = self.selected_audio_files();
        let Some(target_index) = files.iter().position(|file| file.id == id) else {
            return;
        };
        let anchor = self
            .selected_file
            .as_deref()
            .and_then(|selected| files.iter().position(|file| file.id == selected))
            .unwrap_or(target_index);
        let start = anchor.min(target_index);
        let end = anchor.max(target_index);
        let range_ids = files[start..=end]
            .iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        drop(files);

        if !add_to_existing {
            self.selected_file_ids.clear();
        }
        self.selected_file_ids.extend(range_ids);
        self.selected_file = Some(id);
    }

    fn toggle_file_selection(&mut self, id: String) {
        if self.selected_file_ids.contains(&id) && self.selected_file_ids.len() > 1 {
            self.selected_file_ids.remove(&id);
        } else {
            self.selected_file_ids.insert(id.clone());
        }
        self.selected_file = Some(id);
    }

    pub(super) fn refresh_file_path(&mut self, path: &Path) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let parent_id = path_id(parent);
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return false;
        };
        let Some(root_folder) = &mut source.root_folder else {
            return false;
        };
        let Some(parent_folder) = root_folder.find_mut(&parent_id) else {
            return false;
        };
        upsert_file(&mut parent_folder.files, file_entry(&path.to_path_buf()));
        self.folders = vec![root_folder.clone()];
        true
    }

    fn visible_folders(&self) -> Vec<VisibleFolder> {
        let mut folders = Vec::new();
        for folder in &self.folders {
            self.push_visible_folder(folder, 0, &mut folders);
        }
        folders
    }

    fn push_visible_folder(
        &self,
        folder: &FolderEntry,
        depth: usize,
        folders: &mut Vec<VisibleFolder>,
    ) {
        let drag_active = self.drag.is_some();
        let drop_candidate = drag_active && self.can_drop_drag_on_folder(&folder.id);
        folders.push(VisibleFolder {
            id: folder.id.clone(),
            name: folder.name.clone(),
            depth,
            has_children: folder.has_children(),
            expanded: self.is_expanded(&folder.id),
            selected: self.selected_folder == folder.id,
            drag_active,
            drop_candidate,
            drop_target: drop_candidate
                && self.drop_target_folder.as_deref() == Some(folder.id.as_str()),
            rename_draft: self
                .rename_edit
                .as_ref()
                .filter(|edit| edit.folder_id == folder.id)
                .map(|edit| edit.draft.clone()),
            rename_input_id: self
                .rename_edit
                .as_ref()
                .filter(|edit| edit.folder_id == folder.id)
                .map(|edit| edit.input_id),
        });
        if self.is_expanded(&folder.id) {
            for child in &folder.children {
                self.push_visible_folder(child, depth + 1, folders);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceEntry {
    id: String,
    label: String,
    root: PathBuf,
    root_folder: Option<FolderEntry>,
    loading_task: Option<u64>,
}

impl SourceEntry {
    fn new(id: impl Into<String>, label: impl Into<String>, root: PathBuf) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            root,
            root_folder: None,
            loading_task: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderEntry {
    id: String,
    name: String,
    children: Vec<FolderEntry>,
    files: Vec<FileEntry>,
}

impl FolderEntry {
    fn find(&self, id: &str) -> Option<&FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    fn find_mut(&mut self, id: &str) -> Option<&mut FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(id))
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    fn rewrite_path_prefix(&mut self, old_path: &Path, new_path: &Path) {
        self.id = rewrite_path_id(&self.id, old_path, new_path);
        if Path::new(&self.id) == new_path {
            self.name = folder_label(new_path);
        }
        for child in &mut self.children {
            child.rewrite_path_prefix(old_path, new_path);
        }
        for file in &mut self.files {
            file.id = rewrite_path_id(&file.id, old_path, new_path);
        }
    }

    fn rewrite_file_path(&mut self, old_path: &Path, new_path: &Path) -> bool {
        let old_id = path_id(old_path);
        for file in &mut self.files {
            if file.id == old_id {
                *file = file_entry(&new_path.to_path_buf());
                self.files.sort_by(|a, b| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                });
                return true;
            }
        }
        self.children
            .iter_mut()
            .any(|child| child.rewrite_file_path(old_path, new_path))
    }

    fn remove_child_by_id(&mut self, target_id: &str) -> bool {
        if let Some(index) = self.children.iter().position(|child| child.id == target_id) {
            self.children.remove(index);
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.remove_child_by_id(target_id))
    }

    fn take_child_by_id(&mut self, target_id: &str) -> Option<FolderEntry> {
        if let Some(index) = self.children.iter().position(|child| child.id == target_id) {
            return Some(self.children.remove(index));
        }
        self.children
            .iter_mut()
            .find_map(|child| child.take_child_by_id(target_id))
    }

    fn remove_files_by_ids(&mut self, target_ids: &HashSet<String>) -> bool {
        let before = self.files.len();
        self.files.retain(|file| !target_ids.contains(&file.id));
        let mut changed = self.files.len() != before;
        for child in &mut self.children {
            changed |= child.remove_files_by_ids(target_ids);
        }
        changed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FolderRenameEdit {
    folder_id: String,
    draft: String,
    input_id: u64,
    kind: FolderRenameKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FolderRenameKind {
    Rename,
    Create { parent_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileRenameEdit {
    file_id: String,
    draft: String,
    input_id: u64,
    selection_start: usize,
    selection_end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FileColumn {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) width: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct FileColumnResize {
    column_id: String,
    start_x: f32,
    start_width: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FolderBrowserDrag {
    Folder { folder_id: String },
    Files { file_ids: Vec<String> },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct FolderDropResult {
    pub(super) moved_paths: Vec<(PathBuf, PathBuf)>,
    pub(super) status: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FolderDragPreview {
    pub(super) label: String,
    pub(super) pointer: Point,
}

fn default_file_columns() -> Vec<FileColumn> {
    vec![
        file_column("name", "Name", 240.0),
        file_column("extension", "Ext", 54.0),
        file_column("size", "Size", 78.0),
        file_column("modified", "Modified", 112.0),
    ]
}

fn file_column(id: &str, label: &str, width: f32) -> FileColumn {
    FileColumn {
        id: id.to_owned(),
        label: label.to_owned(),
        width,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileRenameView {
    pub(super) draft: String,
    pub(super) input_id: u64,
    pub(super) selection_start: usize,
    pub(super) selection_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderDeleteTargetView {
    pub(super) path: PathBuf,
    pub(super) name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileDeleteTargetView {
    pub(super) paths: Vec<PathBuf>,
    pub(super) names: Vec<String>,
}

impl FileDeleteTargetView {
    pub(super) fn label(&self) -> String {
        match self.names.as_slice() {
            [] => String::from("selected files"),
            [name] => name.clone(),
            names => format!("{} files", names.len()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileEntry {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) stem: String,
    pub(super) extension: String,
    pub(super) kind: String,
    pub(super) size: String,
    pub(super) size_bytes: u64,
    pub(super) modified: String,
    pub(super) modified_rank: u64,
}

impl FileEntry {
    fn is_audio(&self) -> bool {
        self.kind == "Audio"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VisibleFolder {
    id: String,
    name: String,
    depth: usize,
    has_children: bool,
    expanded: bool,
    selected: bool,
    drag_active: bool,
    drop_candidate: bool,
    drop_target: bool,
    rename_draft: Option<String>,
    rename_input_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum FolderBrowserMessage {
    AddSource,
    SelectSource(String),
    ActivateFolder(String),
    DragFolder(String, DragHandleMessage),
    HoverDropTarget(String),
    ClearDropTarget,
    DropOnFolder(String),
    BeginRenameSelected,
    BeginCreateSubfolder,
    RenameInput(TextInputMessage),
    SortFileColumn(String),
    ResizeFileColumn(String, DragHandleMessage),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanRequest {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) label: String,
    pub(super) root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RenameTargetView {
    pub(super) kind: &'static str,
    pub(super) label: String,
    pub(super) is_source_root: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanProgress {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) label: String,
    pub(super) phase: String,
    pub(super) completed: usize,
    pub(super) total: usize,
    pub(super) detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FolderScanItem {
    Folder(FolderEntry),
    File(FileEntry),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanDiscovery {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) parent_id: String,
    pub(super) item: FolderScanItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanDiscoveryBatch {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) events: Vec<FolderScanDiscovery>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanResult {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) label: String,
    pub(super) folder: FolderEntry,
    pub(super) file_count: usize,
    pub(super) folder_count: usize,
}

pub(super) fn folder_browser_view(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::column([
        source_selector(state),
        ui::text("Folders").height(22.0).fill_width(),
        ui::scroll(folder_tree_view(state)).fill(),
        selected_folder_status(state),
    ])
    .spacing(3.0)
    .padding(4.0)
    .style(WidgetStyle::default())
    .fill_height()
}

fn folder_tree_view(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::stack([
        ui::custom_widget_mapped(
            FolderDropClearTarget::new(state.drag.is_some()),
            |message| GuiMessage::FolderBrowser(message),
        )
        .key("folder-drop-clear-target")
        .input_only()
        .fill(),
        ui::column(
            state
                .visible_folders()
                .into_iter()
                .map(folder_row)
                .collect::<Vec<_>>(),
        )
        .fill_width()
        .spacing(1.0),
    ])
    .fill()
}

fn source_selector(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::column([
        ui::row([
            ui::text("Sources").height(20.0).fill_width(),
            ui::button("+")
                .primary()
                .message(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
                .key("source-add-button")
                .size(28.0, 22.0),
        ])
        .spacing(3.0)
        .fill_width()
        .height(24.0),
        ui::column(
            state
                .sources
                .iter()
                .map(|source| source_row(state, source))
                .collect::<Vec<_>>(),
        )
        .spacing(2.0)
        .fill_width(),
    ])
    .spacing(3.0)
    .fill_width()
}

fn source_row(state: &FolderBrowserState, source: &SourceEntry) -> ui::View<GuiMessage> {
    let id = source.id.clone();
    let selected = state.selected_source == source.id;
    let label = if source.loading_task.is_some() {
        format!("{} (scanning)", source.label)
    } else {
        source.label.clone()
    };
    let mut row = ui::button(label)
        .message(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SelectSource(id.clone()),
        ))
        .key(format!("source-row-{id}"))
        .fill_width()
        .height(24.0);
    if selected {
        row = row.primary();
    } else {
        row = row.subtle();
    }
    row.style(if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .fill_width()
}

fn folder_row(folder: VisibleFolder) -> ui::View<GuiMessage> {
    let id = folder.id.clone();
    if let (Some(draft), Some(input_id)) = (folder.rename_draft.clone(), folder.rename_input_id) {
        let caret = draft.chars().count();
        let indent = (folder.depth as f32) * TREE_DEPTH_INDENT;
        return ui::row([
            ui::spacer().width(indent).height(22.0),
            ui::text_input(draft)
                .selection(0, caret)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
                })
                .id(input_id)
                .key(format!("folder-rename-input-{id}"))
                .fill_width()
                .height(22.0),
        ])
        .key(format!("folder-row-{id}"))
        .style(WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .height(TREE_ROW_HEIGHT)
        .spacing(1.0)
        .hoverable();
    }

    let expander = if folder.expanded { "[-]" } else { "[+]" };
    let indent = (folder.depth as f32) * TREE_DEPTH_INDENT;
    let label_text = if folder.has_children {
        format!("{expander} {}", folder.name)
    } else {
        format!("    {}", folder.name)
    };
    let hit_id = id.clone();
    let hit_target = ui::custom_widget_mapped(
        FolderTreeHitTarget::new(
            folder.selected,
            folder.drop_target,
            folder.drag_active,
            folder.drop_candidate,
        ),
        move |message| match message {
            FolderTreeHitMessage::Activate => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(hit_id.clone()))
            }
            FolderTreeHitMessage::Drag(drag) => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(hit_id.clone(), drag))
            }
            FolderTreeHitMessage::Drop => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnFolder(hit_id.clone()))
            }
            FolderTreeHitMessage::HoverDropTarget => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::HoverDropTarget(hit_id.clone()))
            }
        },
    )
    .key(format!("folder-row-hit-{id}"))
    .fill_width()
    .height(22.0);
    let label = ui::text(label_text)
        .key(format!("folder-row-label-{id}"))
        .fill_width()
        .height(22.0)
        .truncate();

    ui::row([
        ui::spacer().width(indent).height(22.0),
        ui::stack([hit_target, label]).fill_width().height(22.0),
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.selected || folder.drop_target {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
    .hoverable()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FolderTreeHitMessage {
    Activate,
    Drag(DragHandleMessage),
    Drop,
    HoverDropTarget,
}

#[derive(Clone, Debug)]
struct FolderTreeHitTarget {
    common: WidgetCommon,
    selected: bool,
    drop_target: bool,
    drag_active: bool,
    drop_candidate: bool,
    dragged: bool,
}

impl FolderTreeHitTarget {
    fn new(selected: bool, drop_target: bool, drag_active: bool, drop_candidate: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            selected,
            drop_target,
            drag_active,
            drop_candidate,
            dragged: false,
        }
    }
}

impl Widget for FolderTreeHitTarget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                if self.common.state.pressed {
                    let message = if self.dragged {
                        DragHandleMessage::Moved { position }
                    } else {
                        self.dragged = true;
                        DragHandleMessage::Started { position }
                    };
                    return Some(WidgetOutput::typed(FolderTreeHitMessage::Drag(message)));
                }
                if self.common.state.hovered && self.drag_active {
                    return Some(WidgetOutput::typed(FolderTreeHitMessage::HoverDropTarget));
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.dragged = false;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated =
                    self.common.state.pressed && !self.dragged && bounds.contains(position);
                let dragged = self.common.state.pressed && self.dragged;
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                self.dragged = false;
                if dragged {
                    return Some(WidgetOutput::typed(FolderTreeHitMessage::Drag(
                        DragHandleMessage::Ended { position },
                    )));
                }
                activated.then(|| WidgetOutput::typed(FolderTreeHitMessage::Activate))
            }
            WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => Some(WidgetOutput::typed(FolderTreeHitMessage::Drop)),
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                    self.dragged = false;
                }
                None
            }
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let mut fill = None;
        if self.drop_target {
            fill = Some(Rgba8 {
                r: 255,
                g: 130,
                b: 78,
                a: 150,
            });
        } else if self.common.state.hovered && self.drop_candidate {
            fill = Some(Rgba8 {
                r: 255,
                g: 122,
                b: 74,
                a: 110,
            });
        } else if self.common.state.pressed || self.common.state.hovered {
            fill = Some(Rgba8 {
                r: 255,
                g: 110,
                b: 85,
                a: if self.common.state.pressed { 120 } else { 80 },
            });
        } else if self.selected {
            fill = Some(Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 105,
            });
        }
        if let Some(color) = fill {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color,
            }));
        }
        if self.drop_target {
            primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
                    Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
                ),
                color: Rgba8 {
                    r: 255,
                    g: 180,
                    b: 130,
                    a: 210,
                },
                width: 1.0,
            }));
        }
    }
}

#[derive(Clone, Debug)]
struct FolderDropClearTarget {
    common: WidgetCommon,
    drag_active: bool,
}

impl FolderDropClearTarget {
    fn new(drag_active: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            drag_active,
        }
    }
}

impl Widget for FolderDropClearTarget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position }
                if self.drag_active && bounds.contains(position) =>
            {
                Some(WidgetOutput::typed(FolderBrowserMessage::ClearDropTarget))
            }
            _ => None,
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

fn selected_folder_status(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let file_count = state.selected_files().len();
    let audio_count = state.selected_audio_files().len();
    let label = state
        .selected_folder()
        .map(|folder| {
            format!(
                "{} | {audio_count} audio | {file_count} item{}",
                folder.name,
                plural(file_count)
            )
        })
        .unwrap_or_else(|| String::from("No folder selected"));
    ui::text(label).height(20.0).fill_width().truncate()
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn default_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn load_root_folder(root: PathBuf) -> FolderEntry {
    load_folder(&root, 0).unwrap_or_else(|| FolderEntry {
        id: path_id(&root),
        name: folder_label(&root),
        children: Vec::new(),
        files: Vec::new(),
    })
}

fn placeholder_folder(root: &Path) -> FolderEntry {
    FolderEntry {
        id: path_id(root),
        name: folder_label(root),
        children: Vec::new(),
        files: Vec::new(),
    }
}

pub(super) fn scan_source_with_progress(
    request: FolderScanRequest,
    mut progress: impl FnMut(FolderScanProgress),
    mut discovered: impl FnMut(FolderScanDiscovery),
) -> FolderScanResult {
    let mut scan = ScanProgressCounter {
        completed: 0,
        files: 0,
        folders: 0,
    };
    progress(FolderScanProgress {
        task_id: request.task_id,
        source_id: request.source_id.clone(),
        label: request.label.clone(),
        phase: String::from("Scanning"),
        completed: 0,
        total: 0,
        detail: request.root.display().to_string(),
    });
    let folder = load_folder_with_progress(
        &request.root,
        0,
        &request,
        &mut scan,
        &mut progress,
        &mut discovered,
    )
    .unwrap_or_else(|| placeholder_folder(&request.root));
    FolderScanResult {
        task_id: request.task_id,
        source_id: request.source_id,
        label: request.label,
        folder,
        file_count: scan.files,
        folder_count: scan.folders,
    }
}

fn load_folder(path: &Path, depth: usize) -> Option<FolderEntry> {
    if depth > MAX_SCAN_DEPTH {
        return None;
    }
    let entries = read_sorted_entries(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .take(MAX_CHILD_FOLDERS)
        .filter_map(|entry| load_folder(entry, depth + 1))
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(file_entry)
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

struct ScanProgressCounter {
    completed: usize,
    files: usize,
    folders: usize,
}

fn load_folder_with_progress(
    path: &Path,
    depth: usize,
    request: &FolderScanRequest,
    scan: &mut ScanProgressCounter,
    progress: &mut impl FnMut(FolderScanProgress),
    discovered: &mut impl FnMut(FolderScanDiscovery),
) -> Option<FolderEntry> {
    if depth > MAX_SCAN_DEPTH {
        return None;
    }
    let entries = read_sorted_entries(path);
    let parent_id = path_id(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .take(MAX_CHILD_FOLDERS)
        .filter_map(|entry| {
            scan.completed += 1;
            scan.folders += 1;
            maybe_report_scan_progress(entry, request, scan, progress);
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::Folder(placeholder_folder(entry)),
            });
            let child =
                load_folder_with_progress(entry, depth + 1, request, scan, progress, discovered)?;
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::Folder(child.clone()),
            });
            Some(child)
        })
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(|entry| {
            scan.completed += 1;
            scan.files += 1;
            maybe_report_scan_progress(entry, request, scan, progress);
            let file = file_entry(entry);
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::File(file.clone()),
            });
            file
        })
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

fn maybe_report_scan_progress(
    path: &Path,
    request: &FolderScanRequest,
    scan: &ScanProgressCounter,
    progress: &mut impl FnMut(FolderScanProgress),
) {
    if scan.completed == 1 || scan.completed.is_multiple_of(64) {
        progress(FolderScanProgress {
            task_id: request.task_id,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Scanning"),
            completed: scan.completed,
            total: 0,
            detail: path.display().to_string(),
        });
    }
}

fn merge_scan_discovery(root: &mut FolderEntry, event: &FolderScanDiscovery) -> bool {
    let Some(parent) = root.find_mut(&event.parent_id) else {
        return false;
    };
    match &event.item {
        FolderScanItem::Folder(folder) => upsert_folder(&mut parent.children, folder.clone()),
        FolderScanItem::File(file) => upsert_file(&mut parent.files, file.clone()),
    }
}

fn upsert_folder(folders: &mut Vec<FolderEntry>, folder: FolderEntry) -> bool {
    match folders.binary_search_by(|candidate| {
        candidate
            .name
            .to_ascii_lowercase()
            .cmp(&folder.name.to_ascii_lowercase())
    }) {
        Ok(index) if folders[index] == folder => false,
        Ok(index) => {
            folders[index] = folder;
            true
        }
        Err(index) => {
            folders.insert(index, folder);
            true
        }
    }
}

fn upsert_file(files: &mut Vec<FileEntry>, file: FileEntry) -> bool {
    match files.binary_search_by(|candidate| {
        candidate
            .name
            .to_ascii_lowercase()
            .cmp(&file.name.to_ascii_lowercase())
    }) {
        Ok(index) if files[index] == file => false,
        Ok(index) => {
            files[index] = file;
            true
        }
        Err(index) => {
            files.insert(index, file);
            true
        }
    }
}

fn file_entry(path: &PathBuf) -> FileEntry {
    let metadata = fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(fs::Metadata::len).unwrap_or_default();
    let modified = metadata.and_then(|metadata| metadata.modified().ok());
    FileEntry {
        id: path_id(path),
        name: file_label(path),
        stem: file_stem_label(path),
        extension: file_extension_label(path),
        kind: file_kind(path),
        size: format_size(size_bytes),
        size_bytes,
        modified: modified_label(modified),
        modified_rank: modified_rank(modified),
    }
}

fn file_kind(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") => String::from("Audio"),
        Some("aif" | "aiff" | "flac" | "mp3") => String::from("Unsupported audio"),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp") => String::from("Image"),
        Some("json" | "txt" | "md" | "toml" | "rs") => String::from("Text"),
        _ => String::from("File"),
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} B")
    }
}

fn modified_label(modified: Option<SystemTime>) -> String {
    let Some(modified) = modified else {
        return String::from("-");
    };
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    let days = age.as_secs() / 86_400;
    if days == 0 {
        String::from("Today")
    } else if days == 1 {
        String::from("1 day")
    } else {
        format!("{days} days")
    }
}

fn modified_rank(modified: Option<SystemTime>) -> u64 {
    modified
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age.as_secs())
        .unwrap_or(u64::MAX)
}

fn read_sorted_entries(path: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        file_label(a)
            .to_ascii_lowercase()
            .cmp(&file_label(b).to_ascii_lowercase())
    });
    entries
}

fn path_id(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn rewrite_path_id(id: &str, old_path: &Path, new_path: &Path) -> String {
    let path = Path::new(id);
    if path == old_path {
        return path_id(new_path);
    }
    path.strip_prefix(old_path)
        .map(|relative| path_id(&new_path.join(relative)))
        .unwrap_or_else(|_| id.to_string())
}

fn valid_folder_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
}

fn valid_file_name(name: &str) -> bool {
    valid_folder_name(name)
}

fn next_available_folder_name(parent: &Path) -> String {
    const BASE_NAME: &str = "New folder";
    if !parent.join(BASE_NAME).exists() {
        return String::from(BASE_NAME);
    }
    (2..)
        .map(|index| format!("{BASE_NAME} {index}"))
        .find(|name| !parent.join(name).exists())
        .unwrap_or_else(|| String::from(BASE_NAME))
}

fn resolved_file_rename(old_path: &Path, submitted: &str) -> Option<String> {
    if submitted.is_empty() {
        return None;
    }
    let submitted_path = Path::new(submitted);
    if submitted_path.components().count() != 1 {
        return None;
    }
    let extension = old_path.extension()?.to_string_lossy();
    Some(format!("{submitted}.{extension}"))
}

fn file_rename_draft(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string())
}

fn rename_input_id(folder_id: &str) -> u64 {
    folder_id
        .bytes()
        .fold(FOLDER_RENAME_INPUT_BASE_ID, |hash, byte| {
            hash.wrapping_mul(16_777_619) ^ u64::from(byte)
        })
}

fn file_rename_input_id(file_id: &str) -> u64 {
    file_id
        .bytes()
        .fold(FILE_RENAME_INPUT_BASE_ID, |hash, byte| {
            hash.wrapping_mul(16_777_619) ^ u64::from(byte)
        })
}

fn folder_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_stem_label(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| file_label(path))
}

fn file_extension_label(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn offset_index(current: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        current
            .saturating_add(delta as usize)
            .min(len.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FolderBrowserMessage, FolderBrowserState, FolderDragPreview, FolderScanDiscoveryBatch,
        MIN_FILE_COLUMN_WIDTH, path_id, scan_source_with_progress,
    };
    use radiant::{
        layout::Point,
        runtime::ExternalDragPayload,
        widgets::{DragHandleMessage, PointerModifiers, TextInputMessage},
    };
    use std::{fs, path::PathBuf};

    #[test]
    fn source_scan_installs_finished_tree_after_placeholder_selection() {
        let root = temp_source_root("radiant-gui-source-scan");
        fs::create_dir_all(root.join("drums")).expect("create nested folder");
        fs::write(root.join("drums").join("kick.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::load_default();
        let request = browser
            .begin_add_source_path(root.clone(), 42)
            .expect("new source should request scan");

        assert_eq!(browser.root_path(), root.as_path());
        assert!(browser.selected_audio_files().is_empty());

        let mut progress_events = Vec::new();
        let mut discovery_events = Vec::new();
        let result = scan_source_with_progress(
            request,
            |progress| progress_events.push(progress),
            |event| discovery_events.push(event),
        );
        assert!(browser.apply_scan_finished(result));

        browser.begin_select_source(root.to_string_lossy().to_string(), 43);
        browser.activate_folder(path_id(&root.join("drums")));
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav"]
        );
        assert!(
            progress_events
                .iter()
                .any(|progress| progress.phase == "Scanning" && progress.total == 0)
        );
        assert!(!discovery_events.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn non_wav_audio_looking_files_are_visible_but_not_supported_audio() {
        let root = temp_source_root("radiant-gui-unsupported-audio");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        for name in ["kick.wav", "loop.aif", "loop.aiff", "loop.flac", "loop.mp3"] {
            fs::write(drums.join(name), [0_u8; 8]).expect("write audio-looking file");
        }

        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav"]
        );

        let unsupported = browser
            .selected_files()
            .iter()
            .filter(|file| file.kind == "Unsupported audio")
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            unsupported,
            vec!["loop.aif", "loop.aiff", "loop.flac", "loop.mp3"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn source_scan_discoveries_populate_selected_tree_before_finish() {
        let root = temp_source_root("radiant-gui-source-streaming");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::load_default();
        let request = browser
            .begin_add_source_path(root.clone(), 77)
            .expect("new source should request scan");

        let mut progress_events = Vec::new();
        let mut discovery_events = Vec::new();
        let result = scan_source_with_progress(
            request,
            |progress| progress_events.push(progress),
            |event| discovery_events.push(event),
        );

        for event in discovery_events {
            browser.apply_scan_discovered(event);
        }
        browser.activate_folder(path_id(&drums));
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav"]
        );

        assert!(browser.apply_scan_finished(result));
        assert!(progress_events.iter().all(|progress| progress.total == 0));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn batched_scan_discoveries_clone_selected_tree_once_per_batch() {
        let root = temp_source_root("radiant-gui-source-batch");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
        fs::write(drums.join("snare.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::load_default();
        let request = browser
            .begin_add_source_path(root.clone(), 88)
            .expect("new source should request scan");

        let mut discovery_events = Vec::new();
        let result =
            scan_source_with_progress(request, |_| {}, |event| discovery_events.push(event));
        assert!(
            browser.apply_scan_discovered_batch(FolderScanDiscoveryBatch {
                task_id: 88,
                source_id: path_id(&root),
                events: discovery_events,
            })
        );
        browser.activate_folder(path_id(&drums));
        assert_eq!(browser.selected_audio_files().len(), 2);

        assert!(browser.apply_scan_finished(result));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn visible_folder_depths_are_stable_for_siblings() {
        let root = temp_source_root("radiant-gui-folder-depths");
        for child in ["alpha", "beta", "gamma"] {
            fs::create_dir_all(root.join("parent").join(child)).expect("create nested folder");
        }
        let browser = FolderBrowserState::from_root(root.clone());
        let mut browser = browser;
        browser.activate_folder(path_id(&root.join("parent")));

        let sibling_depths = browser
            .visible_folders()
            .into_iter()
            .filter(|folder| ["alpha", "beta", "gamma"].contains(&folder.name.as_str()))
            .map(|folder| folder.depth)
            .collect::<Vec<_>>();

        assert_eq!(sibling_depths, vec![2, 2, 2]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_keyboard_navigation_moves_visible_selection_and_expands_collapses() {
        let root = temp_source_root("radiant-gui-folder-keyboard");
        let drums = root.join("drums");
        let kicks = drums.join("kicks");
        let snares = drums.join("snares");
        fs::create_dir_all(&kicks).expect("create kicks folder");
        fs::create_dir_all(&snares).expect("create snares folder");
        let mut browser = FolderBrowserState::from_root(root.clone());

        assert_eq!(browser.selected_folder, path_id(&root));
        assert!(browser.navigate_selected_folder(1));
        assert_eq!(browser.selected_folder, path_id(&drums));
        assert!(!browser.is_expanded(&path_id(&drums)));
        assert!(browser.expand_selected_folder());
        assert!(browser.is_expanded(&path_id(&drums)));
        assert!(browser.collapse_selected_folder());
        assert!(!browser.is_expanded(&path_id(&drums)));
        assert!(browser.expand_selected_folder());
        assert!(browser.is_expanded(&path_id(&drums)));
        assert!(browser.navigate_selected_folder(1));
        assert_eq!(browser.selected_folder, path_id(&kicks));
        assert!(browser.navigate_selected_folder(1));
        assert_eq!(browser.selected_folder, path_id(&snares));
        assert!(!browser.navigate_selected_folder(1));
        assert_eq!(browser.selected_folder, path_id(&snares));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_keyboard_navigation_moves_audio_selection_without_leaving_folder() {
        let root = temp_source_root("radiant-gui-file-keyboard");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let hat = drums.join("hat.wav");
        let kick = drums.join("kick.wav");
        let snare = drums.join("snare.wav");
        fs::write(&hat, [0_u8; 8]).expect("write hat");
        fs::write(&kick, [0_u8; 8]).expect("write kick");
        fs::write(&snare, [0_u8; 8]).expect("write snare");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&hat));

        assert_eq!(browser.navigate_vertical(1, false), Some(path_id(&kick)));
        browser.select_file(path_id(&kick));
        assert_eq!(browser.navigate_vertical(1, false), Some(path_id(&snare)));
        browser.select_file(path_id(&snare));
        assert_eq!(browser.navigate_vertical(1, false), None);
        assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));
        assert_eq!(browser.navigate_vertical(-1, false), Some(path_id(&kick)));
        assert_eq!(browser.selected_folder, path_id(&drums));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_keyboard_navigation_can_extend_audio_selection() {
        let root = temp_source_root("radiant-gui-file-keyboard-extend");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let hat = drums.join("hat.wav");
        let kick = drums.join("kick.wav");
        let snare = drums.join("snare.wav");
        fs::write(&hat, [0_u8; 8]).expect("write hat");
        fs::write(&kick, [0_u8; 8]).expect("write kick");
        fs::write(&snare, [0_u8; 8]).expect("write snare");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&hat));

        assert_eq!(browser.navigate_vertical(1, true), Some(path_id(&kick)));
        assert_eq!(browser.navigate_vertical(1, true), Some(path_id(&snare)));

        assert_eq!(
            browser.selected_file_paths(),
            vec![hat.clone(), kick.clone(), snare.clone()]
        );
        assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_mouse_selection_toggles_and_extends_audio_selection() {
        let root = temp_source_root("radiant-gui-file-mouse-multi-select");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let hat = drums.join("hat.wav");
        let kick = drums.join("kick.wav");
        let snare = drums.join("snare.wav");
        let tom = drums.join("tom.wav");
        for file in [&hat, &kick, &snare, &tom] {
            fs::write(file, [0_u8; 8]).expect("write wav");
        }
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&hat));

        browser.select_file_with_modifiers(
            path_id(&snare),
            PointerModifiers {
                command: true,
                ..Default::default()
            },
        );
        assert_eq!(
            browser.selected_file_paths(),
            vec![hat.clone(), snare.clone()]
        );

        browser.select_file_with_modifiers(
            path_id(&tom),
            PointerModifiers {
                shift: true,
                ..Default::default()
            },
        );
        assert_eq!(
            browser.selected_file_paths(),
            vec![snare.clone(), tom.clone()]
        );

        browser.select_file_with_modifiers(
            path_id(&kick),
            PointerModifiers {
                command: true,
                shift: true,
                ..Default::default()
            },
        );
        assert_eq!(
            browser.selected_file_paths(),
            vec![kick.clone(), snare.clone(), tom.clone()]
        );

        browser.select_file_with_modifiers(
            path_id(&snare),
            PointerModifiers {
                command: true,
                ..Default::default()
            },
        );
        assert_eq!(
            browser.selected_file_paths(),
            vec![kick.clone(), tom.clone()]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_keyboard_navigation_follow_window_moves_only_near_edges() {
        let root = temp_source_root("radiant-gui-file-follow-window");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let files = (0..20)
            .map(|index| drums.join(format!("sample_{index:02}.wav")))
            .collect::<Vec<_>>();
        for file in &files {
            fs::write(file, [0_u8; 8]).expect("write wav");
        }
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&files[4]));

        let window = browser.follow_selected_file_view(6, 1, 1);
        assert_eq!(window.viewport_start, 1);
        assert_eq!(browser.file_view_start(), 1);

        assert_eq!(
            browser.navigate_vertical(1, false),
            Some(path_id(&files[5]))
        );
        let window = browser.follow_selected_file_view(6, 1, 1);
        assert_eq!(window.viewport_start, 2);
        assert_eq!(browser.file_view_start(), 2);

        assert_eq!(
            browser.navigate_vertical(1, false),
            Some(path_id(&files[6]))
        );
        let window = browser.follow_selected_file_view(6, 1, 1);
        assert_eq!(window.viewport_start, 3);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_scroll_tracking_allows_runtime_clamped_bottom_offsets() {
        let root = temp_source_root("radiant-gui-file-scroll-bottom");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let files = (0..24)
            .map(|index| drums.join(format!("sample_{index:02}.wav")))
            .collect::<Vec<_>>();
        for file in &files {
            fs::write(file, [0_u8; 8]).expect("write wav");
        }
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        browser.set_file_view_start_from_scroll_offset(23.0 * 22.0, 22.0);

        assert_eq!(browser.file_view_start(), 23);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn select_all_audio_files_selects_current_folder_samples() {
        let root = temp_source_root("radiant-gui-file-select-all");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let hat = drums.join("hat.wav");
        let kick = drums.join("kick.wav");
        let note = drums.join("note.txt");
        fs::write(&hat, [0_u8; 8]).expect("write hat");
        fs::write(&kick, [0_u8; 8]).expect("write kick");
        fs::write(&note, [0_u8; 8]).expect("write note");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        assert_eq!(browser.select_all_audio_files(), 2);

        assert_eq!(
            browser.selected_file_paths(),
            vec![hat.clone(), kick.clone()]
        );
        assert!(!browser.is_file_selected(&path_id(&note)));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sample_file_sort_toggles_by_column_and_navigation_uses_sorted_order() {
        let root = temp_source_root("radiant-gui-file-sort");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let small = drums.join("small.wav");
        let large = drums.join("large.wav");
        fs::write(&small, [0_u8; 8]).expect("write small");
        fs::write(&large, [0_u8; 128]).expect("write large");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from("size")));
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["small.wav", "large.wav"]
        );

        browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from("size")));
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["large.wav", "small.wav"]
        );
        browser.select_file(path_id(&large));
        assert_eq!(browser.navigate_vertical(1, false), Some(path_id(&small)));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sample_file_column_resize_clamps_width() {
        let mut browser = FolderBrowserState::load_default();

        browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
            String::from("extension"),
            radiant::widgets::DragHandleMessage::Started {
                position: Point::new(100.0, 0.0),
            },
        ));
        browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
            String::from("extension"),
            radiant::widgets::DragHandleMessage::Moved {
                position: Point::new(-200.0, 0.0),
            },
        ));

        let extension_width = browser
            .visible_file_columns()
            .into_iter()
            .find(|column| column.id == "extension")
            .map(|column| column.width)
            .unwrap();
        assert_eq!(extension_width, MIN_FILE_COLUMN_WIDTH);
    }

    #[test]
    fn browser_keyboard_navigation_is_disabled_while_renaming() {
        let root = temp_source_root("radiant-gui-keyboard-rename");
        let drums = root.join("drums");
        let kicks = drums.join("kicks");
        fs::create_dir_all(&kicks).expect("create kicks folder");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser
            .begin_rename_selected()
            .expect("rename can start")
            .expect("rename input id");

        assert!(browser.rename_active());
        assert_eq!(browser.navigate_vertical(1, false), None);
        assert!(!browser.expand_selected_folder());
        assert!(!browser.collapse_selected_folder());
        assert_eq!(browser.selected_folder, path_id(&drums));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_rename_updates_filesystem_tree_and_selected_audio_files() {
        let root = temp_source_root("radiant-gui-folder-rename");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        let input_id = browser
            .begin_rename_selected()
            .expect("rename can start")
            .expect("rename input id");
        assert_ne!(input_id, 0);
        let status = browser
            .apply_rename_input(TextInputMessage::Submitted {
                value: String::from("breaks"),
            })
            .expect("rename status");

        assert_eq!(status, "Renamed folder to breaks");
        assert!(!drums.exists());
        assert!(root.join("breaks").join("kick.wav").is_file());
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.id.as_str())
                .collect::<Vec<_>>(),
            vec![path_id(&root.join("breaks").join("kick.wav"))]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_subfolder_starts_pending_rename_row_and_creates_on_submit() {
        let root = temp_source_root("radiant-gui-folder-create");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        let input_id = browser
            .begin_create_subfolder()
            .expect("create can start")
            .expect("rename input id");
        let pending = drums.join("New folder");

        assert_ne!(input_id, 0);
        assert!(!pending.exists());
        assert!(browser.is_expanded(&path_id(&drums)));
        assert!(
            browser.visible_folders().into_iter().any(|folder| {
                folder.id == path_id(&pending)
                    && folder.selected
                    && folder.rename_draft.as_deref() == Some("New folder")
                    && folder.rename_input_id == Some(input_id)
            }),
            "expected pending child rename row"
        );

        let status = browser
            .apply_rename_input(TextInputMessage::Submitted {
                value: String::from("loops"),
            })
            .expect("create status");

        assert_eq!(status, "Created folder loops");
        assert!(!pending.exists());
        assert!(drums.join("loops").is_dir());
        assert_eq!(browser.selected_folder, path_id(&drums.join("loops")));
        assert!(
            browser
                .visible_folders()
                .into_iter()
                .any(|folder| folder.id == path_id(&drums.join("loops"))
                    && folder.name == "loops"
                    && folder.rename_draft.is_none())
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_subfolder_cancel_removes_pending_row_without_touching_disk() {
        let root = temp_source_root("radiant-gui-folder-create-cancel");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        browser
            .begin_create_subfolder()
            .expect("create can start")
            .expect("rename input id");
        let pending = drums.join("New folder");

        browser.activate_folder(path_id(&drums));

        assert!(!pending.exists());
        assert_eq!(browser.selected_folder, path_id(&drums));
        assert!(
            browser
                .visible_folders()
                .into_iter()
                .all(|folder| folder.id != path_id(&pending))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_subfolder_default_name_skips_existing_folder() {
        let root = temp_source_root("radiant-gui-folder-create-unique");
        let drums = root.join("drums");
        fs::create_dir_all(drums.join("New folder")).expect("create existing folder");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));

        browser
            .begin_create_subfolder()
            .expect("create can start")
            .expect("rename input id");

        assert!(
            browser.visible_folders().into_iter().any(|folder| {
                folder.id == path_id(&drums.join("New folder 2"))
                    && folder.rename_draft.as_deref() == Some("New folder 2")
            }),
            "expected unique default name"
        );
        assert!(!drums.join("New folder 2").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_rename_hides_and_preserves_extension() {
        let root = temp_source_root("radiant-gui-file-rename");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        let kick = drums.join("kick loop.wav");
        fs::write(&kick, [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&kick));

        let input_id = browser
            .begin_rename_selected()
            .expect("rename can start")
            .expect("rename input id");
        let rename = browser
            .file_rename_view(&path_id(&kick))
            .expect("file rename view");
        assert_eq!(rename.input_id, input_id);
        assert_eq!(rename.draft, "kick loop");
        assert_eq!(rename.selection_start, 0);
        assert_eq!(rename.selection_end, "kick loop".chars().count());

        let status = browser
            .apply_rename_input(TextInputMessage::Submitted {
                value: String::from("snare loop"),
            })
            .expect("rename status");

        assert_eq!(status, "Renamed file to snare loop.wav");
        assert!(!kick.exists());
        assert!(drums.join("snare loop.wav").is_file());
        assert_eq!(
            browser.selected_file_id(),
            Some(path_id(&drums.join("snare loop.wav")).as_str())
        );
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| (
                    file.name.as_str(),
                    file.stem.as_str(),
                    file.extension.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![("snare loop.wav", "snare loop", "wav")]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_rename_submission_cannot_change_extension() {
        let root = temp_source_root("radiant-gui-file-rename-extension");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        let kick = drums.join("kick.wav");
        fs::write(&kick, [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&kick));
        browser
            .begin_rename_selected()
            .expect("rename can start")
            .expect("rename input id");

        let status = browser
            .apply_rename_input(TextInputMessage::Submitted {
                value: String::from("snare.aiff"),
            })
            .expect("rename status");

        assert_eq!(status, "Renamed file to snare.aiff.wav");
        assert!(!kick.exists());
        assert!(drums.join("snare.aiff.wav").is_file());
        assert!(!drums.join("snare.aiff").exists());
        assert_eq!(
            browser.selected_file_id(),
            Some(path_id(&drums.join("snare.aiff.wav")).as_str())
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn root_folder_rename_is_rejected_from_tree() {
        let root = temp_source_root("radiant-gui-root-rename");
        let mut browser = FolderBrowserState::from_root(root.clone());

        assert_eq!(
            browser.begin_rename_selected(),
            Err(String::from("Select a subfolder to rename"))
        );
        assert!(root.is_dir());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_delete_blocks_hard_delete_and_keeps_selected_folder() {
        let root = temp_source_root("radiant-gui-folder-delete");
        let drums = root.join("drums");
        let kicks = drums.join("kicks");
        fs::create_dir_all(&kicks).expect("create nested folder");
        fs::write(kicks.join("kick.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.expand_selected_folder();
        browser.activate_folder(path_id(&kicks));

        let target = browser
            .selected_delete_target()
            .expect("subfolder can be deleted");
        assert_eq!(target.name, "kicks");
        let error = browser
            .delete_selected_folder()
            .expect_err("hard delete should be blocked");

        assert_eq!(
            error,
            "Trash workflow is not available in the default GUI yet; no folder was deleted"
        );
        assert!(kicks.exists());
        assert_eq!(browser.selected_folder, path_id(&kicks));
        assert!(browser.find_folder(&path_id(&kicks)).is_some());
        assert!(browser.expanded_folders.contains(&path_id(&drums)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_delete_blocks_hard_delete_and_keeps_selection() {
        let root = temp_source_root("radiant-gui-file-delete");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create drums folder");
        let hat = drums.join("hat.wav");
        let kick = drums.join("kick.wav");
        let snare = drums.join("snare.wav");
        for file in [&hat, &kick, &snare] {
            fs::write(file, [0_u8; 8]).expect("write wav");
        }
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&kick));

        let target = browser
            .selected_file_delete_target()
            .expect("selected file can be deleted");
        assert_eq!(target.names, vec![String::from("kick.wav")]);
        let error = browser
            .delete_selected_files()
            .expect_err("hard delete should be blocked");

        assert_eq!(
            error,
            "Trash workflow is not available in the default GUI yet; no files were deleted"
        );
        assert!(kick.exists());
        assert!(
            browser
                .selected_files()
                .iter()
                .any(|file| file.name == "kick.wav")
        );
        assert_eq!(browser.selected_file_id(), Some(path_id(&kick).as_str()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_drag_drop_moves_subtree_into_target_folder() {
        let root = temp_source_root("radiant-gui-folder-drag-drop");
        let kicks = root.join("drums").join("kicks");
        let loops = root.join("loops");
        fs::create_dir_all(&kicks).expect("create kicks folder");
        fs::create_dir_all(&loops).expect("create loops folder");
        let kick = kicks.join("kick.wav");
        fs::write(&kick, [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&root.join("drums")));
        browser.expand_selected_folder();
        browser.activate_folder(path_id(&kicks));
        browser.select_file(path_id(&kick));

        browser.apply_folder_drag(
            path_id(&kicks),
            DragHandleMessage::Started {
                position: Point::new(0.0, 0.0),
            },
        );
        let result = browser
            .drop_drag_on_folder(&path_id(&loops))
            .expect("folder drag/drop should move");

        let moved_kicks = loops.join("kicks");
        let moved_kick = moved_kicks.join("kick.wav");
        assert_eq!(
            result.moved_paths,
            vec![(kicks.clone(), moved_kicks.clone())]
        );
        assert!(!kicks.exists());
        assert!(moved_kick.is_file());
        assert_eq!(browser.selected_folder, path_id(&moved_kicks));
        assert_eq!(
            browser.selected_file_id(),
            Some(path_id(&moved_kick).as_str())
        );
        assert!(browser.find_folder(&path_id(&moved_kicks)).is_some());
        assert!(browser.expanded_folders.contains(&path_id(&loops)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_drag_preview_tracks_pointer_and_hover_target() {
        let root = temp_source_root("radiant-gui-folder-drag-preview");
        let kicks = root.join("drums").join("kicks");
        let loops = root.join("loops");
        fs::create_dir_all(&kicks).expect("create kicks folder");
        fs::create_dir_all(&loops).expect("create loops folder");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&root.join("drums")));
        browser.expand_selected_folder();

        browser.apply_folder_drag(
            path_id(&kicks),
            DragHandleMessage::Started {
                position: Point::new(10.0, 20.0),
            },
        );
        assert_eq!(
            browser.drag_preview(),
            Some(FolderDragPreview {
                label: String::from("kicks"),
                pointer: Point::new(10.0, 20.0),
            })
        );

        browser.apply_folder_drag(
            path_id(&kicks),
            DragHandleMessage::Moved {
                position: Point::new(30.0, 42.0),
            },
        );
        assert_eq!(
            browser.drag_preview().map(|preview| preview.pointer),
            Some(Point::new(30.0, 42.0))
        );

        browser.apply_message(FolderBrowserMessage::HoverDropTarget(path_id(&loops)));
        let hovered = browser
            .visible_folders()
            .into_iter()
            .find(|folder| folder.id == path_id(&loops))
            .expect("loops folder visible");
        assert!(hovered.drop_candidate);
        assert!(hovered.drop_target);

        browser.apply_message(FolderBrowserMessage::HoverDropTarget(path_id(&kicks)));
        assert!(
            browser
                .visible_folders()
                .into_iter()
                .all(|folder| !folder.drop_target)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_drag_external_request_uses_preview_label_and_paths() {
        let root = temp_source_root("radiant-gui-folder-external-drag");
        let kicks = root.join("drums").join("kicks");
        fs::create_dir_all(&kicks).expect("create kicks folder");
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&root.join("drums")));
        browser.expand_selected_folder();

        browser.apply_folder_drag(
            path_id(&kicks),
            DragHandleMessage::Started {
                position: Point::new(10.0, 20.0),
            },
        );
        let request = browser
            .external_drag_request()
            .expect("folder drag should expose external request");

        assert_eq!(request.preview.label, "kicks");
        assert_eq!(
            request.payload,
            ExternalDragPayload::Files(vec![kicks.clone()])
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_drag_drop_moves_selected_files_into_target_folder() {
        let root = temp_source_root("radiant-gui-file-drag-drop");
        let drums = root.join("drums");
        let loops = root.join("loops");
        fs::create_dir_all(&drums).expect("create drums folder");
        fs::create_dir_all(&loops).expect("create loops folder");
        let kick = drums.join("kick.wav");
        let snare = drums.join("snare.wav");
        let hat = drums.join("hat.wav");
        for file in [&kick, &snare, &hat] {
            fs::write(file, [0_u8; 8]).expect("write wav");
        }
        let mut browser = FolderBrowserState::from_root(root.clone());
        browser.activate_folder(path_id(&drums));
        browser.select_file(path_id(&kick));
        browser.select_file_with_modifiers(
            path_id(&snare),
            PointerModifiers {
                command: true,
                ..Default::default()
            },
        );

        browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
        let result = browser
            .drop_drag_on_folder(&path_id(&loops))
            .expect("file drag/drop should move");

        let moved_kick = loops.join("kick.wav");
        let moved_snare = loops.join("snare.wav");
        assert_eq!(result.moved_paths.len(), 2);
        assert!(!kick.exists());
        assert!(!snare.exists());
        assert!(hat.is_file());
        assert!(moved_kick.is_file());
        assert!(moved_snare.is_file());
        assert_eq!(browser.selected_folder, path_id(&loops));
        assert_eq!(
            browser.selected_file_paths(),
            vec![moved_kick.clone(), moved_snare.clone()]
        );
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav", "snare.wav"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn root_folder_delete_is_rejected_from_tree() {
        let root = temp_source_root("radiant-gui-root-delete");
        let browser = FolderBrowserState::from_root(root.clone());

        assert_eq!(
            browser.selected_delete_target(),
            Err(String::from("Root folder cannot be deleted"))
        );
        assert!(root.is_dir());
        let _ = fs::remove_dir_all(root);
    }

    fn temp_source_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
