use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextTargetKind, target_label,
};
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS, FOLDER_TREE_SELECTION_CONTEXT_ROWS, TREE_ROW_HEIGHT,
};

mod worker;
use worker::create_unique_child_folder;

impl NativeAppState {
    pub(in crate::native_app) fn create_folder_at_context_target(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if !matches!(
            menu.kind,
            BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder
        ) {
            self.ui.status.sample = String::from("Choose a folder to add a subfolder");
            emit_gui_action(
                "folder_browser.context_menu.new_folder",
                Some("folder_browser"),
                None,
                "blocked",
                started_at,
                Some("unsupported target"),
            );
            return;
        }

        let parent = menu.path;
        if let Some(error) = self
            .library
            .folder_browser
            .folder_target_lock_error(&parent, "New folder")
        {
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "folder_browser.context_menu.new_folder",
                Some("folder_browser"),
                Some(target_label(&parent).as_str()),
                "blocked",
                started_at,
                Some(&error),
            );
            return;
        }
        if let Some(source_id) = menu.source_id {
            self.select_source(source_id, context);
        }
        let parent_id = parent.display().to_string();
        self.ui.status.sample = format!("Creating folder in {}", target_label(&parent));
        context.business().blocking_io("gui-folder-create").run(
            {
                let parent = parent.clone();
                move |_| create_unique_child_folder(&parent)
            },
            move |result| GuiMessage::ContextFolderCreateFinished {
                parent_id,
                started_at,
                result,
            },
        );
    }

    pub(in crate::native_app) fn finish_context_folder_create(
        &mut self,
        parent_id: String,
        started_at: Instant,
        result: Result<PathBuf, String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let new_path = match result {
            Ok(path) => path,
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "folder_browser.context_menu.new_folder",
                    Some("folder_browser"),
                    Some(parent_id.as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };

        match self
            .library
            .folder_browser
            .apply_created_folder(parent_id.clone(), new_path.clone())
        {
            Ok(input_id) => {
                self.library.folder_browser.sync_tree_view_to_selection(
                    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
                    FOLDER_TREE_OVERSCAN_ROWS,
                    FOLDER_TREE_EDGE_CONTEXT_ROWS,
                );
                if let Some(index) = self.library.folder_browser.selected_folder_visible_index() {
                    context.scroll_fixed_row_into_view(
                        FOLDER_TREE_LIST_ID,
                        index,
                        TREE_ROW_HEIGHT,
                        FOLDER_TREE_SELECTION_CONTEXT_ROWS,
                        FOLDER_TREE_SELECTION_CONTEXT_ROWS,
                        1,
                    );
                }
                context.after(
                    Duration::from_millis(1),
                    GuiMessage::FocusRenameInput(input_id),
                );
                let name = new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| String::from("New Folder"));
                self.ui.status.sample = format!("Created folder {name}");
                emit_gui_action(
                    "folder_browser.context_menu.new_folder",
                    Some("folder_browser"),
                    Some(parent_id.as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "folder_browser.context_menu.new_folder",
                    Some("folder_browser"),
                    Some(parent_id.as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}
