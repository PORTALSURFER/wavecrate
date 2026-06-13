use std::{path::PathBuf, time::Instant};

use super::movement::{move_path_to_configured_trash, move_paths_to_configured_trash};
use crate::native_app::app::{
    GuiMessage, NativeAppState, TrashMoveTarget, emit_gui_action, sample_path_label,
};
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;

impl NativeAppState {
    pub(in crate::native_app) fn move_context_target_to_trash(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        match menu.kind {
            BrowserContextTargetKind::Folder => {
                self.move_folder_path_to_trash(
                    menu.path,
                    "browser.context_menu.folder.trash",
                    started_at,
                    context,
                );
            }
            BrowserContextTargetKind::Sample => {
                self.move_file_paths_to_trash(
                    vec![menu.path],
                    "browser.context_menu.sample.trash",
                    started_at,
                    context,
                );
            }
            BrowserContextTargetKind::Source | BrowserContextTargetKind::MetadataTag => {
                self.ui.status.sample = String::from("Context target cannot be moved to trash");
                emit_gui_action(
                    "browser.context_menu.trash",
                    Some("browser"),
                    None,
                    "blocked",
                    started_at,
                    Some("unsupported target"),
                );
            }
        }
    }

    pub(in crate::native_app) fn move_selected_folder_to_trash(
        &mut self,
        path: PathBuf,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        self.move_folder_path_to_trash(path, "folder_browser.delete_selected", started_at, context);
    }

    pub(in crate::native_app) fn move_selected_files_to_trash(
        &mut self,
        paths: Vec<PathBuf>,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        self.move_file_paths_to_trash(paths, "browser.delete_selected_files", started_at, context);
    }

    fn move_folder_path_to_trash(
        &mut self,
        path: PathBuf,
        action: &'static str,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let trash_folder = self.ui.settings.persisted.trash_folder.clone();
        self.ui.status.sample = format!("Moving {} to trash", sample_path_label(&path));
        context.business().background("gui-trash-move").run(
            {
                let path = path.clone();
                move |_| {
                    move_path_to_configured_trash(&path, trash_folder.as_deref())
                        .map(|destination| vec![destination])
                }
            },
            move |result| GuiMessage::TrashMoveFinished {
                target: TrashMoveTarget::Folder(path),
                action,
                started_at,
                result,
            },
        );
    }

    pub(in crate::native_app) fn finish_trash_move(
        &mut self,
        target: TrashMoveTarget,
        action: &'static str,
        started_at: Instant,
        result: Result<Vec<PathBuf>, String>,
    ) {
        match (target, result) {
            (TrashMoveTarget::Folder(path), Ok(moved)) => {
                let destination = moved.first().cloned().unwrap_or_else(|| path.clone());
                self.finish_folder_trash_move(path, destination, action, started_at);
            }
            (TrashMoveTarget::Files(paths), Ok(moved)) => {
                self.finish_file_trash_move(paths, moved.len(), action, started_at);
            }
            (TrashMoveTarget::Folder(path), Err(error)) => {
                self.finish_trash_move_error(Some(path), action, started_at, error);
            }
            (TrashMoveTarget::Files(_), Err(error)) => {
                self.finish_trash_move_error(None, action, started_at, error);
            }
        }
    }

    fn finish_folder_trash_move(
        &mut self,
        path: PathBuf,
        destination: PathBuf,
        action: &'static str,
        started_at: Instant,
    ) {
        self.library
            .folder_browser
            .discard_trashed_folder_path(&path);
        self.clear_loaded_sample_if_path_within(&path);
        self.ui.status.sample = format!("Moved {} to trash", sample_path_label(&destination));
        emit_gui_action(
            action,
            Some("folder_browser"),
            Some(sample_path_label(&path).as_str()),
            "success",
            started_at,
            None,
        );
    }

    fn finish_file_trash_move(
        &mut self,
        paths: Vec<PathBuf>,
        moved_count: usize,
        action: &'static str,
        started_at: Instant,
    ) {
        self.library
            .folder_browser
            .discard_trashed_file_paths(&paths);
        for path in &paths {
            self.clear_loaded_sample_if_exact(path);
        }
        let noun = if moved_count == 1 { "file" } else { "files" };
        self.ui.status.sample = format!("Moved {moved_count} {noun} to trash");
        emit_gui_action(
            action,
            Some("browser"),
            Some(&format!("{moved_count} {noun}")),
            "success",
            started_at,
            None,
        );
    }

    fn finish_trash_move_error(
        &mut self,
        path: Option<PathBuf>,
        action: &'static str,
        started_at: Instant,
        error: String,
    ) {
        self.ui.status.sample = error.clone();
        emit_gui_action(
            action,
            Some("browser"),
            path.as_ref().map(sample_path_label).as_deref(),
            "error",
            started_at,
            Some(&error),
        );
    }

    fn move_file_paths_to_trash(
        &mut self,
        paths: Vec<PathBuf>,
        action: &'static str,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let trash_folder = self.ui.settings.persisted.trash_folder.clone();
        self.ui.status.sample = match paths.len() {
            1 => String::from("Moving file to trash"),
            count => format!("Moving {count} files to trash"),
        };
        context.business().background("gui-trash-move").run(
            {
                let paths = paths.clone();
                move |_| move_paths_to_configured_trash(&paths, trash_folder.as_deref())
            },
            move |result| GuiMessage::TrashMoveFinished {
                target: TrashMoveTarget::Files(paths),
                action,
                started_at,
                result,
            },
        );
    }
}
