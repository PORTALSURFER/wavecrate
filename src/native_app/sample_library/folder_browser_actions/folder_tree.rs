use radiant::prelude as ui;
use radiant::widgets::{DragHandleMessage, PointerModifiers};
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_OVERSCAN_ROWS, FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
};
use crate::native_app::sample_library::source_prep::{
    CacheWarmIntent, MetadataRefreshIntent, ReadinessIntent, SourceFeedbackIntent,
    SourcePrepIntents, SourcePriorityIntent,
};

pub(in crate::native_app) const FOLDER_ACTIVATION_PREP_INTENTS: SourcePrepIntents =
    SourcePrepIntents {
        readiness: ReadinessIntent::RequestConvergence,
        priority: SourcePriorityIntent::PromoteIfSelected,
        metadata_refresh: MetadataRefreshIntent::IfNotLoaded,
        refresh_waveform_cache_projection_if_selected: true,
        cache_warm: CacheWarmIntent::Preserve,
        feedback: SourceFeedbackIntent::Preserve,
    };
pub(in crate::native_app) const FOLDER_ACTIVATION_PREP_REASON: &str = "folder_activated";

impl NativeAppState {
    pub(super) fn activate_folder_browser_folder(
        &mut self,
        folder_id: String,
        modifiers: PointerModifiers,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let source = folder_id.clone();
        if let Some((sample_source, relative_path)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(std::path::Path::new(&folder_id))
        {
            self.background
                .source_processing
                .set_current_folder(sample_source.id.as_str(), &relative_path.to_string_lossy());
        }
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::ActivateFolder(folder_id, modifiers));
        self.queue_selected_folder_verify_after_activation(context);
        self.queue_selected_source_prep(
            FOLDER_ACTIVATION_PREP_INTENTS,
            FOLDER_ACTIVATION_PREP_REASON,
            context,
        );
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

    pub(super) fn drop_on_folder_browser_source(
        &mut self,
        source_id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.ui.browser_interaction.context_menu = None;
        self.drop_browser_drag_on_source(source_id, context);
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
