use std::path::{Path, PathBuf};

use radiant::prelude as ui;

use crate::native_app::app::NativeAppState;

impl NativeAppState {
    pub(in crate::native_app) fn arm_pending_internal_file_drag_paths(
        &mut self,
        request: Option<&ui::ExternalDragRequest>,
        add_keep_rating: bool,
    ) {
        self.ui
            .browser_interaction
            .pending_internal_file_drag_paths
            .clear();
        self.ui
            .browser_interaction
            .pending_internal_file_drag_adds_keep_rating = false;
        let Some(ui::ExternalDragPayload::Files(paths)) = request.map(|request| &request.payload)
        else {
            return;
        };
        self.ui
            .browser_interaction
            .pending_internal_file_drag_paths
            .extend(paths.iter().map(|path| normalized_drag_path(path)));
        self.ui
            .browser_interaction
            .pending_internal_file_drag_adds_keep_rating = add_keep_rating;
    }

    pub(in crate::native_app) fn clear_pending_internal_file_drag_paths(&mut self) {
        self.ui
            .browser_interaction
            .pending_internal_file_drag_paths
            .clear();
        self.ui
            .browser_interaction
            .pending_internal_file_drag_adds_keep_rating = false;
    }

    pub(in crate::native_app) fn is_pending_internal_file_drag_path(&self, path: &Path) -> bool {
        self.ui
            .browser_interaction
            .pending_internal_file_drag_paths
            .contains(&normalized_drag_path(path))
    }
}

fn normalized_drag_path(path: &Path) -> PathBuf {
    path.components().collect()
}
