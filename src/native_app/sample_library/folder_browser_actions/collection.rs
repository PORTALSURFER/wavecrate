use radiant::prelude as ui;
use std::time::{Duration, Instant};
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;

impl NativeAppState {
    pub(super) fn activate_folder_browser_collection(
        &mut self,
        collection: SampleCollection,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::ActivateCollection(collection));
        self.schedule_persisted_waveform_cache_indicator_refresh(context);
        self.cancel_active_folder_cache_warm();
    }

    pub(super) fn drop_on_folder_browser_collection(
        &mut self,
        collection: SampleCollection,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.ui.browser_interaction.context_menu = None;
        self.drop_drag_on_collection(collection, context);
    }

    pub(super) fn begin_collection_rename(
        &mut self,
        collection: SampleCollection,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self
            .library
            .folder_browser
            .begin_rename_collection(collection)
        {
            Some(input_id) => {
                self.ui.status.sample = String::from("Renaming collection");
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
                self.ui.status.sample = String::from("Select a collection to rename");
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
}
