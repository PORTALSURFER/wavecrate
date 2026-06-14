use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::file_actions::sample_path_label;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS, TREE_ROW_HEIGHT,
};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT,
};

impl NativeAppState {
    pub(super) fn apply_folder_browser_tag_filter_input(
        &mut self,
        message: radiant::widgets::TextInputMessage,
    ) {
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::TagFilterInput(message));
        self.library
            .folder_browser
            .retain_visible_file_selection_after_tag_filter(&self.metadata.tags_by_file);
    }

    pub(super) fn toggle_similarity_anchor(
        &mut self,
        file_id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let clearing = self
            .library
            .folder_browser
            .file_is_similarity_anchor(&file_id);
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::ToggleSimilarityAnchor(
                file_id.clone(),
            ));
        if clearing {
            self.ui.status.sample = String::from("Similarity mode cleared");
            emit_gui_action(
                "browser.similarity_anchor.clear",
                Some("browser"),
                Some(&sample_path_label(&file_id)),
                "applied",
                started_at,
                None,
            );
        } else {
            context.scroll_into_view_snapped(
                SAMPLE_BROWSER_LIST_ID,
                0.0,
                SAMPLE_BROWSER_ROW_HEIGHT,
                0.0,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_ROW_HEIGHT,
            );
            self.ui.status.sample = format!(
                "Similarity anchor set to {}",
                sample_path_label(file_id.as_str())
            );
            emit_gui_action(
                "browser.similarity_anchor.set",
                Some("browser"),
                Some(&sample_path_label(&file_id)),
                "applied",
                started_at,
                None,
            );
        }
    }

    pub(in crate::native_app) fn focus_rename_input(
        &mut self,
        input_id: u64,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.focus(input_id);
        emit_gui_action(
            "folder_browser.rename.focus_input",
            Some("folder_browser"),
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn select_all_samples(&mut self) {
        let started_at = Instant::now();
        let count = self
            .library
            .folder_browser
            .select_all_audio_files_matching_tags(&self.metadata.tags_by_file);
        self.ui.status.sample = format!(
            "Selected {count} sample{}",
            if count == 1 { "" } else { "s" }
        );
        emit_gui_action(
            "browser.select_all_samples",
            Some("browser"),
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn toggle_selected_sample_and_advance(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let previous_focus = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let Some(result) = self
            .library
            .folder_browser
            .toggle_focused_sample_selection_and_advance(&self.metadata.tags_by_file)
        else {
            self.ui.status.sample = String::from("Select a sample to mark");
            emit_gui_action(
                "browser.toggle_sample_selection_and_advance",
                Some("browser"),
                None,
                "short_circuit",
                started_at,
                None,
            );
            return;
        };

        if self.library.folder_browser.selected_file_id() != previous_focus.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        if let Some(index) = self
            .library
            .folder_browser
            .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
        {
            context.scroll_fixed_row_into_view(
                SAMPLE_BROWSER_LIST_ID,
                index,
                SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                1,
            );
        }

        let action = if result.toggled_selected {
            "Marked"
        } else {
            "Unmarked"
        };
        let count = self.library.folder_browser.selected_audio_file_count();
        self.ui.status.sample = format!(
            "{action} {} ({count} selected)",
            sample_path_label(&result.toggled_id)
        );
        emit_gui_action(
            "browser.toggle_sample_selection_and_advance",
            Some("browser"),
            Some(&sample_path_label(&result.toggled_id)),
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn navigate_browser(
        &mut self,
        delta: i32,
        extend: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let direction = if delta < 0 { "previous" } else { "next" };
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let previous_folder = self
            .library
            .folder_browser
            .selected_folder_id()
            .map(str::to_owned);
        let Some(path) = self.library.folder_browser.navigate_vertical_matching_tags(
            delta,
            extend,
            &self.metadata.tags_by_file,
        ) else {
            if self
                .library
                .folder_browser
                .selected_folder_id()
                .is_some_and(|folder| Some(folder) != previous_folder.as_deref())
            {
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
                        FOLDER_TREE_EDGE_CONTEXT_ROWS,
                        FOLDER_TREE_EDGE_CONTEXT_ROWS,
                        delta,
                    );
                }
                emit_gui_action(
                    "folder_browser.navigate",
                    Some("folder_browser"),
                    Some(direction),
                    "selected_folder",
                    started_at,
                    None,
                );
                return;
            }
            emit_gui_action(
                "folder_browser.navigate",
                Some("browser"),
                Some(direction),
                "edge",
                started_at,
                None,
            );
            return;
        };

        if let Some(index) = self
            .library
            .folder_browser
            .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
        {
            context.scroll_fixed_row_into_view(
                SAMPLE_BROWSER_LIST_ID,
                index,
                SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                delta,
            );
        }
        emit_gui_action(
            "folder_browser.navigate",
            Some("browser"),
            Some(direction),
            "selected",
            started_at,
            None,
        );
        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.defer_navigation_sample_load(path, context);
    }
}
