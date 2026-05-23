use radiant::gui::types::Point;
use std::time::Instant;

use super::context_menu::{self, BrowserContextMenu, BrowserContextTargetKind};
use super::file_actions::{
    format_copy_path, open_folder_in_file_explorer, reveal_in_file_explorer, sample_path_label,
};
use super::{FolderBrowserMessage, GuiAppState, emit_gui_action};
use wavecrate::external_clipboard;

impl GuiAppState {
    pub(super) fn open_source_context_menu(&mut self, source_id: String, position: Point) {
        let started_at = Instant::now();
        let Some(path) = self.folder_browser.source_root_path(&source_id) else {
            self.sample_status = String::from("Source is unavailable");
            emit_gui_action(
                "browser.context_menu.source.open",
                Some("sources"),
                None,
                "error",
                started_at,
                Some("source unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Source, &path) {
            self.sample_status = String::from("Source folder is missing");
            emit_gui_action(
                "browser.context_menu.source.open",
                Some("sources"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("source folder missing"),
            );
            return;
        }
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Source,
            path,
            anchor: position,
            title,
        });
    }

    pub(super) fn open_folder_context_menu(&mut self, folder_id: String, position: Point) {
        let started_at = Instant::now();
        self.folder_browser
            .apply_message(FolderBrowserMessage::ActivateFolder(folder_id.clone()));
        let Some(path) = self.folder_browser.folder_path(&folder_id) else {
            self.sample_status = String::from("Folder is unavailable");
            emit_gui_action(
                "browser.context_menu.folder.open",
                Some("folder_browser"),
                None,
                "error",
                started_at,
                Some("folder unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Folder, &path) {
            self.sample_status = String::from("Folder is missing");
            emit_gui_action(
                "browser.context_menu.folder.open",
                Some("folder_browser"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("folder missing"),
            );
            return;
        }
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Folder,
            path,
            anchor: position,
            title,
        });
    }

    pub(super) fn open_sample_context_menu(&mut self, path: String, position: Point) {
        let started_at = Instant::now();
        self.folder_browser
            .focus_file_preserving_selection(path.clone());
        let Some(path) = self.folder_browser.context_sample_path(&path) else {
            self.sample_status = String::from("Sample is unavailable");
            emit_gui_action(
                "browser.context_menu.sample.open",
                Some("browser"),
                None,
                "error",
                started_at,
                Some("sample unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Sample, &path) {
            self.sample_status = String::from("Sample file is missing");
            emit_gui_action(
                "browser.context_menu.sample.open",
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("sample missing"),
            );
            return;
        }
        let title = sample_path_label(&path);
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Sample,
            path,
            anchor: position,
            title,
        });
    }

    pub(super) fn copy_context_path(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.copy_path",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let path_text = format_copy_path(&menu.path);
        match external_clipboard::copy_text(&path_text) {
            Ok(()) => {
                self.sample_status = String::from("Copied path");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("Copy path failed: {error}");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn open_context_target(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.open_explorer",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let result = match menu.kind {
            BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => {
                open_folder_in_file_explorer(&menu.path)
            }
            BrowserContextTargetKind::Sample => reveal_in_file_explorer(&menu.path),
        };
        match result {
            Ok(()) => {
                self.sample_status = match menu.kind {
                    BrowserContextTargetKind::Sample => String::from("Revealed sample"),
                    BrowserContextTargetKind::Source => String::from("Opened source folder"),
                    BrowserContextTargetKind::Folder => String::from("Opened folder"),
                };
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&menu.kind)),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}
