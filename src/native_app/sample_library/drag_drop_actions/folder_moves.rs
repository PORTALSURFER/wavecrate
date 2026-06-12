use std::{path::PathBuf, time::Instant};

use radiant::prelude as ui;

use crate::native_app::app::{
    FileMoveConflictResolution, GuiMessage, NativeAppState, emit_gui_action,
};

impl NativeAppState {
    pub(in crate::native_app) fn drop_browser_drag_on_folder(
        &mut self,
        folder_id: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.end_drag_session();
        self.clear_pending_internal_file_drag_paths();
        match self.library.folder_browser.drop_drag_on_folder(&folder_id) {
            Ok(result) => {
                self.apply_moved_sample_paths(&result.moved_paths);
                if let Some(status) = result.status {
                    self.ui.status.sample = status;
                }
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    if result.moved_paths.is_empty() {
                        "unchanged"
                    } else {
                        "success"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                self.library.folder_browser.clear_drag();
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn resolve_file_move_conflict(
        &mut self,
        resolution: FileMoveConflictResolution,
    ) {
        let started_at = Instant::now();
        match self
            .library
            .folder_browser
            .resolve_next_file_move_conflict(resolution)
        {
            Ok(result) => {
                self.apply_moved_sample_paths(&result.moved_paths);
                if let Some(status) = result.status {
                    self.ui.status.sample = status;
                }
                emit_gui_action(
                    "browser.drag_drop.file_conflict.resolve",
                    Some("browser"),
                    None,
                    if result.moved_paths.is_empty() {
                        "skipped"
                    } else {
                        "success"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.drag_drop.file_conflict.resolve",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn cancel_file_move_conflicts(&mut self) {
        if let Some(status) = self.library.folder_browser.cancel_file_move_conflicts() {
            self.ui.status.sample = status;
        }
    }

    pub(in crate::native_app) fn apply_moved_sample_paths(
        &mut self,
        moved_paths: &[(PathBuf, PathBuf)],
    ) {
        for (old_path, new_path) in moved_paths {
            self.waveform
                .current
                .rewrite_path_prefix(old_path, new_path);
            self.remap_renamed_waveform_cache_path(old_path, new_path);
        }
    }
}
