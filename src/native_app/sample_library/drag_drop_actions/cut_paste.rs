use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::app::{CutFileClipboard, GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::commands::FolderMoveDropInput;

impl NativeAppState {
    pub(in crate::native_app) fn cut_selected_files(&mut self) {
        let started_at = Instant::now();
        let paths = self.library.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.ui.status.sample = String::from("Select files before cutting");
            emit_gui_action(
                "browser.cut_selected_files",
                Some("browser"),
                None,
                "skipped",
                started_at,
                Some("no selection"),
            );
            return;
        }

        let file_ids = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        let count = file_ids.len();
        self.ui.browser_interaction.cut_file_clipboard = Some(CutFileClipboard::new(file_ids));
        self.ui.browser_interaction.cut_file_paste_task_id = None;
        self.library.folder_browser.flash_copied_file_paths(&paths);
        self.ui.status.sample = match count {
            1 => String::from("Cut selected file"),
            count => format!("Cut {count} selected files"),
        };
        emit_gui_action(
            "browser.cut_selected_files",
            Some("browser"),
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn paste_cut_files(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(clipboard) = self.ui.browser_interaction.cut_file_clipboard.clone() else {
            self.ui.status.sample = String::from("Cut files before pasting");
            emit_gui_action(
                "browser.paste_cut_files",
                Some("browser"),
                None,
                "skipped",
                started_at,
                Some("empty cut buffer"),
            );
            return;
        };
        if self.ui.browser_interaction.cut_file_paste_task_id.is_some() {
            let label = cut_file_count_label(clipboard.len());
            self.ui.status.sample = String::from("Pasting cut files");
            emit_gui_action(
                "browser.paste_cut_files",
                Some("browser"),
                Some(&label),
                "skipped",
                started_at,
                Some("paste already queued"),
            );
            return;
        }
        let Some(target_folder_id) = self
            .library
            .folder_browser
            .selected_folder_id()
            .map(str::to_owned)
        else {
            self.ui.status.sample = String::from("Select a folder before pasting files");
            emit_gui_action(
                "browser.paste_cut_files",
                Some("browser"),
                None,
                "skipped",
                started_at,
                Some("no target folder"),
            );
            return;
        };

        let result = self
            .library
            .folder_browser
            .prepare_paste_cut_files_to_folder(&clipboard.file_ids, &target_folder_id);
        match result {
            Ok(input) => {
                let label = cut_file_count_label(clipboard.len());
                let move_requested = matches!(input, FolderMoveDropInput::Request(_));
                if let Some(task_id) = self.submit_folder_move_input(input, started_at, context) {
                    self.ui.browser_interaction.cut_file_paste_task_id = Some(task_id);
                }
                emit_gui_action(
                    "browser.paste_cut_files",
                    Some("browser"),
                    Some(&label),
                    if move_requested {
                        "queued"
                    } else {
                        "unchanged"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                let label = cut_file_count_label(clipboard.len());
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.paste_cut_files",
                    Some("browser"),
                    Some(&label),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}

fn cut_file_count_label(count: usize) -> String {
    match count {
        1 => String::from("1 file"),
        count => format!("{count} files"),
    }
}
