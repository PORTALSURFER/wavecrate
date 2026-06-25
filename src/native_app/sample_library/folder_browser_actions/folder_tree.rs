use radiant::prelude as ui;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_OVERSCAN_ROWS, FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
};
use crate::native_app::sample_library::source_prep::SourcePrepTrigger;

impl NativeAppState {
    pub(super) fn activate_folder_browser_folder(
        &mut self,
        folder_id: String,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let source = folder_id.clone();
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::ActivateFolder(folder_id, modifiers));
        self.queue_selected_folder_verify_after_activation(context);
        self.queue_selected_source_prep(SourcePrepTrigger::FolderActivated, context);
        emit_gui_action(
            "folder_browser.activate_folder",
            Some("folder_browser"),
            Some(source.as_str()),
            "applied",
            started_at,
            None,
        );
    }

    pub(super) fn drop_on_folder_browser_folder(
        &mut self,
        folder_id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.ui.browser_interaction.context_menu = None;
        self.drop_browser_drag_on_folder(folder_id, context);
    }

    pub(super) fn drag_folder_browser_folder(
        &mut self,
        folder_id: String,
        drag: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.ui.browser_interaction.context_menu = None;
        self.drag_folder(folder_id, drag, context);
    }

    pub(in crate::native_app) fn collapse_selected_folder(&mut self) {
        let started_at = Instant::now();
        self.library.folder_browser.collapse_selected_folder();
        self.library.folder_browser.sync_tree_view_to_selection(
            FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
            FOLDER_TREE_OVERSCAN_ROWS,
            FOLDER_TREE_EDGE_CONTEXT_ROWS,
        );
        emit_gui_action(
            "folder_browser.collapse_selected",
            Some("folder_browser"),
            None,
            "success",
            started_at,
            None,
        );
    }
}
