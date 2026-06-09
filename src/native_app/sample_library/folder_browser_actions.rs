use radiant::prelude as ui;
use radiant::widgets::DragHandleMessage;
use std::time::{Duration, Instant};
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::file_actions::sample_path_label;
use crate::native_app::sample_library::folder_browser::{
    FolderBrowserMessage, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH,
};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT,
};

impl NativeAppState {
    pub(in crate::native_app) fn resize_folder_browser(&mut self, message: DragHandleMessage) {
        let started_at = Instant::now();
        let phase = message.phase();
        let should_log = !message.is_moved();
        let outcome = phase.as_str();
        self.folder_panel.resize(
            message,
            ui::PanelResizeConstraints::right(MIN_FOLDER_WIDTH, MAX_FOLDER_WIDTH),
        );
        if should_log {
            emit_gui_action(
                "layout.resize_folder_browser",
                Some("folder_browser"),
                None,
                outcome,
                started_at,
                None,
            );
        }
    }

    pub(in crate::native_app) fn apply_folder_browser_message(
        &mut self,
        message: FolderBrowserMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            FolderBrowserMessage::AddSource => self.add_source_from_dialog(context),
            FolderBrowserMessage::SelectSource(id) => {
                let started_at = Instant::now();
                let source = id.clone();
                self.context_menu = None;
                self.select_source(id, context);
                self.schedule_active_folder_cache_warm(context);
                emit_gui_action(
                    "folder_browser.select_source",
                    Some("folder_browser"),
                    Some(source.as_str()),
                    "applied",
                    started_at,
                    None,
                );
            }
            FolderBrowserMessage::OpenSourceContextMenu(source_id, position) => {
                self.open_source_context_menu(source_id, position);
            }
            FolderBrowserMessage::BeginRenameSelected => self.begin_folder_browser_rename(context),
            FolderBrowserMessage::CancelRename => {
                self.folder_browser.cancel_rename();
            }
            FolderBrowserMessage::BeginCreateSubfolder => {
                self.begin_folder_browser_subfolder_creation(context);
            }
            FolderBrowserMessage::RenameInput(message) => {
                self.apply_folder_browser_rename_input(message);
            }
            FolderBrowserMessage::TagFilterInput(message) => {
                self.folder_browser
                    .apply_message(FolderBrowserMessage::TagFilterInput(message));
                self.folder_browser
                    .retain_visible_file_selection_after_tag_filter(&self.metadata.tags_by_file);
            }
            FolderBrowserMessage::DropOnFolder(folder_id) => {
                self.context_menu = None;
                self.drop_browser_drag_on_folder(folder_id, context);
            }
            FolderBrowserMessage::DropOnCollection(collection) => {
                self.context_menu = None;
                self.drop_drag_on_collection(collection, context);
            }
            FolderBrowserMessage::OpenFolderContextMenu(folder_id, position) => {
                self.open_folder_context_menu(folder_id, position);
            }
            FolderBrowserMessage::ActivateFolder(folder_id) => {
                let started_at = Instant::now();
                let source = folder_id.clone();
                self.folder_browser
                    .apply_message(FolderBrowserMessage::ActivateFolder(folder_id));
                self.schedule_persisted_waveform_cache_indicator_refresh(context);
                self.schedule_active_folder_cache_warm(context);
                emit_gui_action(
                    "folder_browser.activate_folder",
                    Some("folder_browser"),
                    Some(source.as_str()),
                    "applied",
                    started_at,
                    None,
                );
            }
            FolderBrowserMessage::DragFolder(folder_id, drag) => {
                self.context_menu = None;
                self.drag_folder(folder_id, drag, context);
            }
            FolderBrowserMessage::ActivateCollection(collection) => {
                self.folder_browser
                    .apply_message(FolderBrowserMessage::ActivateCollection(collection));
                self.schedule_persisted_waveform_cache_indicator_refresh(context);
                self.cancel_active_folder_cache_warm();
            }
            FolderBrowserMessage::RenameCollection(collection) => {
                self.begin_collection_rename(collection, context);
            }
            message => self.folder_browser.apply_message(message),
        }
    }

    fn begin_collection_rename(
        &mut self,
        collection: SampleCollection,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self.folder_browser.begin_rename_collection(collection) {
            Some(input_id) => {
                self.sample_status = String::from("Renaming collection");
                context.after(
                    Duration::from_millis(1),
                    GuiMessage::FocusRenameInput(input_id),
                );
                emit_gui_action(
                    "folder_browser.collection.rename.begin",
                    Some("folder_browser"),
                    Some("collection"),
                    "success",
                    started_at,
                    None,
                );
            }
            None => {
                self.sample_status = String::from("Select a collection to rename");
                emit_gui_action(
                    "folder_browser.collection.rename.begin",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some("collection_missing"),
                );
            }
        }
    }

    pub(in crate::native_app) fn focus_rename_input(
        &mut self,
        input_id: u64,
        context: &mut ui::UpdateContext<GuiMessage>,
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
            .folder_browser
            .select_all_audio_files_matching_tags(&self.metadata.tags_by_file);
        self.sample_status = format!(
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
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let previous_focus = self.folder_browser.selected_file_id().map(str::to_owned);
        let Some(result) = self
            .folder_browser
            .toggle_focused_sample_selection_and_advance(&self.metadata.tags_by_file)
        else {
            self.sample_status = String::from("Select a sample to mark");
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

        if self.folder_browser.selected_file_id() != previous_focus.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        if let Some(index) = self
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
        let count = self.folder_browser.selected_audio_file_count();
        self.sample_status = format!(
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

    pub(in crate::native_app) fn collapse_selected_folder(&mut self) {
        let started_at = Instant::now();
        self.folder_browser.collapse_selected_folder();
        emit_gui_action(
            "folder_browser.collapse_selected",
            Some("folder_browser"),
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn expand_selected_folder(&mut self) {
        let started_at = Instant::now();
        self.folder_browser.expand_selected_folder();
        emit_gui_action(
            "folder_browser.expand_selected",
            Some("folder_browser"),
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn navigate_browser(
        &mut self,
        delta: i32,
        extend: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let direction = if delta < 0 { "previous" } else { "next" };
        let previous_selection = self.folder_browser.selected_file_id().map(str::to_owned);
        let Some(path) = self.folder_browser.navigate_vertical_matching_tags(
            delta,
            extend,
            &self.metadata.tags_by_file,
        ) else {
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
        if self.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.defer_navigation_sample_load(path, context);
    }
}
