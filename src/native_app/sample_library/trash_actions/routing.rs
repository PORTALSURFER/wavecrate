use std::{path::PathBuf, time::Instant};

use super::movement::{move_path_to_configured_trash, move_paths_to_configured_trash};
use crate::native_app::app::{NativeAppState, emit_gui_action, sample_path_label};
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;

impl NativeAppState {
    pub(in crate::native_app) fn move_context_target_to_trash(&mut self) {
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
                );
            }
            BrowserContextTargetKind::Sample => {
                self.move_file_paths_to_trash(
                    vec![menu.path],
                    "browser.context_menu.sample.trash",
                    started_at,
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
    ) {
        self.move_folder_path_to_trash(path, "folder_browser.delete_selected", started_at);
    }

    pub(in crate::native_app) fn move_selected_files_to_trash(
        &mut self,
        paths: Vec<PathBuf>,
        started_at: Instant,
    ) {
        self.move_file_paths_to_trash(paths, "browser.delete_selected_files", started_at);
    }

    fn move_folder_path_to_trash(
        &mut self,
        path: PathBuf,
        action: &'static str,
        started_at: Instant,
    ) {
        match move_path_to_configured_trash(
            &path,
            self.ui.settings.persisted.trash_folder.as_deref(),
        ) {
            Ok(destination) => {
                self.library
                    .folder_browser
                    .discard_trashed_folder_path(&path);
                self.clear_loaded_sample_if_path_within(&path);
                self.ui.status.sample =
                    format!("Moved {} to trash", sample_path_label(&destination));
                emit_gui_action(
                    action,
                    Some("folder_browser"),
                    Some(sample_path_label(&path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    action,
                    Some("folder_browser"),
                    Some(sample_path_label(&path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn move_file_paths_to_trash(
        &mut self,
        paths: Vec<PathBuf>,
        action: &'static str,
        started_at: Instant,
    ) {
        match move_paths_to_configured_trash(
            &paths,
            self.ui.settings.persisted.trash_folder.as_deref(),
        ) {
            Ok(moved) => {
                self.library
                    .folder_browser
                    .discard_trashed_file_paths(&paths);
                for path in &paths {
                    self.clear_loaded_sample_if_exact(path);
                }
                let count = moved.len();
                let noun = if count == 1 { "file" } else { "files" };
                self.ui.status.sample = format!("Moved {count} {noun} to trash");
                emit_gui_action(
                    action,
                    Some("browser"),
                    Some(&format!("{count} {noun}")),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    action,
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}
