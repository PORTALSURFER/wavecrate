use super::*;
use std::collections::HashSet;
use std::time::{Duration, Instant};

const FOLDER_DRAG_HOVER_EXPAND_DELAY: Duration = Duration::from_millis(450);

#[derive(Clone, Debug)]
pub(in crate::native_app::sample_library::folder_browser) struct BrowserDragDropState {
    pub(in crate::native_app::sample_library::folder_browser) drag: Option<FolderBrowserDrag>,
    pub(in crate::native_app::sample_library::folder_browser) drag_pointer: Option<Point>,
    pub(in crate::native_app::sample_library::folder_browser) drop_target:
        ui::ExclusiveOpen<FolderBrowserDropTarget>,
    pub(in crate::native_app::sample_library::folder_browser) pending_file_move_conflicts:
        Option<super::super::FileMoveConflictBatch>,
    folder_hover_auto_expand: Option<FolderHoverAutoExpand>,
}

impl BrowserDragDropState {
    pub(in crate::native_app::sample_library::folder_browser) fn new() -> Self {
        Self {
            drag: None,
            drag_pointer: None,
            drop_target: ui::ExclusiveOpen::new(),
            pending_file_move_conflicts: None,
            folder_hover_auto_expand: None,
        }
    }

    pub(super) fn arm_folder_hover_auto_expand(&mut self, folder_id: &str, started_at: Instant) {
        if self
            .folder_hover_auto_expand
            .as_ref()
            .is_some_and(|pending| pending.folder_id == folder_id)
        {
            return;
        }
        self.folder_hover_auto_expand = Some(FolderHoverAutoExpand::new(folder_id, started_at));
    }

    pub(in crate::native_app::sample_library::folder_browser) fn clear_folder_hover_auto_expand(
        &mut self,
    ) {
        self.folder_hover_auto_expand = None;
    }

    pub(super) fn folder_hover_auto_expand_pending(&self) -> bool {
        self.folder_hover_auto_expand.is_some()
    }

    pub(super) fn folder_hover_auto_expand_ready_id(&self, now: Instant) -> Option<&str> {
        self.folder_hover_auto_expand
            .as_ref()
            .filter(|pending| pending.ready_at(now))
            .map(|pending| pending.folder_id.as_str())
    }
}

#[derive(Clone, Debug)]
struct FolderHoverAutoExpand {
    folder_id: String,
    started_at: Instant,
}

impl FolderHoverAutoExpand {
    fn new(folder_id: &str, started_at: Instant) -> Self {
        Self {
            folder_id: folder_id.to_owned(),
            started_at,
        }
    }

    fn ready_at(&self, now: Instant) -> bool {
        now.checked_duration_since(self.started_at)
            .is_some_and(|elapsed| elapsed >= FOLDER_DRAG_HOVER_EXPAND_DELAY)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FolderBrowserDropTarget {
    Folder(String),
    Source(String),
    Collection(SampleCollection),
}

impl FolderBrowserState {
    pub(in crate::native_app) fn begin_file_drag(
        &mut self,
        file_id: String,
        position: Point,
    ) -> bool {
        if self.rename_active()
            || !self
                .selected_audio_files()
                .iter()
                .any(|file| file.id == file_id)
        {
            return false;
        }
        let file_ids = if self.selection.selected_file_ids_explicit
            && !self.selection.selected_file_ids.is_empty()
        {
            sorted_selected_file_ids(&self.selection.selected_file_ids)
        } else if self.selection.selected_file_ids.contains(&file_id) {
            let mut ids = self
                .selection
                .selected_file_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            ids.sort();
            ids
        } else {
            vec![file_id]
        };
        self.drag_drop.drag = Some(FolderBrowserDrag::Files {
            file_ids,
            remove_from_collection: None,
        });
        self.drag_drop.drag_pointer = Some(position);
        self.clear_drop_targets_for_new_drag();
        true
    }

    pub(in crate::native_app) fn begin_extracted_file_drag(
        &mut self,
        path: PathBuf,
        position: Point,
    ) {
        if self.rename_active() {
            return;
        }
        self.drag_drop.drag = Some(FolderBrowserDrag::ExtractedFile { path });
        self.drag_drop.drag_pointer = Some(position);
        self.clear_drop_targets_for_new_drag();
    }

    pub(in crate::native_app) fn update_drag_pointer(&mut self, position: Point) {
        if self.drag_drop.drag.is_some() {
            self.drag_drop.drag_pointer = Some(position);
        }
    }

    pub(in crate::native_app) fn clear_drag(&mut self) {
        self.clear_source_reorder_drag();
        self.drag_drop.drag = None;
        self.drag_drop.drag_pointer = None;
        self.drag_drop.drop_target.close();
        self.drag_drop.folder_hover_auto_expand = None;
    }

    pub(in crate::native_app) fn drag_hover_auto_expand_pending(&self) -> bool {
        self.drag_drop.folder_hover_auto_expand_pending()
    }

    pub(in crate::native_app) fn advance_drag_hover_folder_auto_expand(&mut self) -> bool {
        self.advance_drag_hover_folder_auto_expand_at(Instant::now())
    }

    pub(in crate::native_app::sample_library::folder_browser) fn advance_drag_hover_folder_auto_expand_at(
        &mut self,
        now: Instant,
    ) -> bool {
        let Some(folder_id) = self
            .drag_drop
            .folder_hover_auto_expand_ready_id(now)
            .map(str::to_owned)
        else {
            return false;
        };

        let still_hovered = self.drag_drop.drop_target.current().is_some_and(
            |target| matches!(target, FolderBrowserDropTarget::Folder(current) if current == &folder_id),
        );
        if !still_hovered || !self.drag_hover_folder_can_auto_expand(&folder_id) {
            self.drag_drop.clear_folder_hover_auto_expand();
            return false;
        }

        self.tree.expanded_folders.insert(folder_id);
        self.drag_drop.clear_folder_hover_auto_expand();
        true
    }

    pub(in crate::native_app::sample_library::folder_browser) fn drag_hover_folder_can_auto_expand(
        &self,
        folder_id: &str,
    ) -> bool {
        self.can_drop_drag_on_folder(folder_id)
            && self.folder_has_children(folder_id)
            && !self.is_expanded(folder_id)
    }

    pub(in crate::native_app::sample_library::folder_browser) fn apply_folder_drag(
        &mut self,
        folder_id: String,
        message: DragHandleMessage,
    ) {
        if self.rename_active() {
            return;
        }
        if let Some(position) = message.started_position() {
            if self.selected_folder_is_source_root_id(&folder_id) {
                return;
            }
            if self.find_folder(&folder_id).is_some() {
                let folder_ids = self.folder_ids_for_drag(folder_id);
                if folder_ids.is_empty() {
                    return;
                }
                self.drag_drop.drag = Some(FolderBrowserDrag::Folder { folder_ids });
                self.drag_drop.drag_pointer = Some(position);
                self.clear_drop_targets_for_new_drag();
            }
        } else if let Some(position) = message.moved_position() {
            self.update_drag_pointer(position);
        } else if message.is_finished() {
            self.clear_drag();
        }
    }

    fn folder_ids_for_drag(&self, folder_id: String) -> Vec<String> {
        let mut ids = if self.selection.selected_folder_ids.contains(&folder_id) {
            self.selection
                .selected_folder_ids
                .iter()
                .filter(|id| !self.selected_folder_is_source_root_id(id))
                .filter(|id| self.find_folder(id).is_some())
                .cloned()
                .collect::<Vec<_>>()
        } else {
            vec![folder_id]
        };
        ids.sort();
        prune_nested_folder_ids(ids)
    }
}

fn sorted_selected_file_ids(selected_file_ids: &HashSet<String>) -> Vec<String> {
    let mut ids = selected_file_ids.iter().cloned().collect::<Vec<_>>();
    ids.sort();
    ids
}

fn prune_nested_folder_ids(folder_ids: Vec<String>) -> Vec<String> {
    let mut roots = Vec::new();
    for folder_id in folder_ids {
        let folder_path = Path::new(&folder_id);
        if roots
            .iter()
            .any(|root: &String| folder_path.starts_with(Path::new(root)))
        {
            continue;
        }
        roots.push(folder_id);
    }
    roots
}
