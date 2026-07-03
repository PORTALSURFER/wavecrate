use super::state::FolderBrowserDropTarget;
use super::*;
use std::time::Instant;

impl FolderBrowserState {
    pub(in crate::native_app) fn clear_drop_target_folder(&mut self, position: Point) {
        self.update_drag_pointer(position);
        if self
            .drag_drop
            .drop_target
            .current()
            .is_some_and(|target| matches!(target, FolderBrowserDropTarget::Folder(_)))
        {
            self.drag_drop.drop_target.close();
            self.drag_drop.clear_folder_hover_auto_expand();
        }
    }

    pub(in crate::native_app) fn clear_drop_target_source(&mut self, position: Point) {
        self.update_drag_pointer(position);
        if self
            .drag_drop
            .drop_target
            .current()
            .is_some_and(|target| matches!(target, FolderBrowserDropTarget::Source(_)))
        {
            self.drag_drop.drop_target.close();
        }
    }

    pub(in crate::native_app) fn clear_drop_target_folder_unless(
        &mut self,
        retained_folder_id: &str,
        position: Point,
    ) {
        self.update_drag_pointer(position);
        if self
            .drag_drop
            .drop_target
            .current()
            .is_some_and(
                |target| matches!(target, FolderBrowserDropTarget::Folder(folder_id) if folder_id == retained_folder_id),
            )
        {
            return;
        }
        self.clear_drop_target_folder(position);
    }

    pub(in crate::native_app) fn clear_drop_target_source_unless(
        &mut self,
        retained_source_id: &str,
        position: Point,
    ) {
        self.update_drag_pointer(position);
        if self
            .drag_drop
            .drop_target
            .current()
            .is_some_and(
                |target| matches!(target, FolderBrowserDropTarget::Source(source_id) if source_id == retained_source_id),
            )
        {
            return;
        }
        self.clear_drop_target_source(position);
    }

    pub(in crate::native_app) fn hovered_drop_target_folder_id(&self) -> Option<String> {
        match self.drag_drop.drop_target.current() {
            Some(FolderBrowserDropTarget::Folder(folder_id)) => Some(folder_id.clone()),
            _ => None,
        }
    }

    pub(in crate::native_app) fn hovered_drop_target_source_id(&self) -> Option<String> {
        match self.drag_drop.drop_target.current() {
            Some(FolderBrowserDropTarget::Source(source_id)) => Some(source_id.clone()),
            _ => None,
        }
    }

    pub(in crate::native_app::sample_library::folder_browser) fn hover_drop_target_folder(
        &mut self,
        folder_id: &str,
    ) {
        self.hover_drop_target_folder_at(folder_id, Instant::now());
    }

    pub(in crate::native_app::sample_library::folder_browser) fn hover_drop_target_folder_at(
        &mut self,
        folder_id: &str,
        now: Instant,
    ) {
        if self.can_drop_drag_on_folder(folder_id) {
            self.drag_drop
                .drop_target
                .open(FolderBrowserDropTarget::Folder(folder_id.to_owned()));
        } else {
            self.drag_drop.drop_target.close();
        }
        if self.drag_hover_folder_can_auto_expand(folder_id) {
            self.drag_drop.arm_folder_hover_auto_expand(folder_id, now);
        } else {
            self.drag_drop.clear_folder_hover_auto_expand();
        }
    }

    pub(in crate::native_app::sample_library::folder_browser) fn hover_drop_target_source(
        &mut self,
        source_id: &str,
    ) {
        if self.can_drop_drag_on_source(source_id) {
            self.drag_drop
                .drop_target
                .open(FolderBrowserDropTarget::Source(source_id.to_owned()));
        } else {
            self.drag_drop.drop_target.close();
        }
        self.drag_drop.clear_folder_hover_auto_expand();
    }

    pub(super) fn clear_drop_targets_for_new_drag(&mut self) {
        self.drag_drop.clear_folder_hover_auto_expand();
        self.drag_drop.drop_target.close();
    }
}
