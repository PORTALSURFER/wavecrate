mod command;
mod persistence;
mod state_application;
mod status;
mod transaction;

use radiant::prelude as ui;
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::view_contract::{
    MissingCollectionFile, collection_hotkey,
};

use command::{CollectionCommand, CollectionSourcePath, CollectionUpdate};

impl NativeAppState {
    pub(in crate::native_app) fn assign_selected_collection(
        &mut self,
        collection: SampleCollection,
        _context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let updates = self.collection_updates_for_selected_files(collection);
        self.apply_collection_updates(collection, updates, "hotkey", CollectionCommand::Toggle);
    }

    pub(in crate::native_app) fn remove_context_sample_from_collection(
        &mut self,
        _context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        let Some(collection) = menu.collection else {
            self.ui.status.sample = String::from("Sample is not in the active collection");
            return;
        };
        let updates = self
            .library
            .folder_browser
            .context_file_collection_candidate(&menu.path, collection)
            .and_then(|candidate| {
                self.collection_update_for_candidate(
                    candidate,
                    collection,
                    CollectionCommand::Remove,
                )
            })
            .into_iter()
            .collect();
        self.apply_collection_updates(
            collection,
            updates,
            "context_menu",
            CollectionCommand::Remove,
        );
    }

    pub(in crate::native_app) fn clean_missing_context_sample_from_collection(&mut self) {
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        let Some(collection) = menu.collection else {
            self.ui.status.sample = String::from("Missing sample is not in the active collection");
            return;
        };
        let files = self
            .library
            .folder_browser
            .missing_collection_file_for_path(&menu.path, collection)
            .into_iter()
            .collect();
        self.apply_missing_collection_cleanup(collection, files, "context_menu");
    }

    pub(in crate::native_app) fn clean_missing_files_from_active_collection(&mut self) {
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        let Some(collection) = menu
            .collection
            .or_else(|| self.library.folder_browser.active_collection())
        else {
            self.ui.status.sample = String::from("No active collection to clean");
            return;
        };
        let files = self
            .library
            .folder_browser
            .missing_collection_files_for_collection(collection);
        self.apply_missing_collection_cleanup(collection, files, "context_menu_all");
    }

    pub(in crate::native_app) fn drop_drag_on_collection(
        &mut self,
        collection: SampleCollection,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let updates = self.collection_updates_for_dragged_files(collection);
        context.end_drag_session();
        self.library.folder_browser.clear_drag();
        self.apply_collection_updates(collection, updates, "drop", CollectionCommand::Add);
    }

    fn collection_updates_for_selected_files(
        &self,
        collection: SampleCollection,
    ) -> Vec<CollectionUpdate> {
        self.library
            .folder_browser
            .selected_file_collection_candidates(collection)
            .into_iter()
            .filter_map(|candidate| {
                self.collection_update_for_candidate(
                    candidate,
                    collection,
                    CollectionCommand::Toggle,
                )
            })
            .collect()
    }

    fn collection_updates_for_dragged_files(
        &self,
        collection: SampleCollection,
    ) -> Vec<CollectionUpdate> {
        self.library
            .folder_browser
            .drag_file_collection_candidates(collection)
            .into_iter()
            .filter_map(|candidate| {
                self.collection_update_for_candidate(candidate, collection, CollectionCommand::Add)
            })
            .collect()
    }

    fn collection_update_for_candidate(
        &self,
        candidate: crate::native_app::sample_library::folder_browser::view_contract::SelectedFileCollectionCandidate,
        collection: SampleCollection,
        command: CollectionCommand,
    ) -> Option<CollectionUpdate> {
        let (root, database_root, relative_path) = self
            .library
            .folder_browser
            .source_database_relative_file_path(&candidate.path)?;
        command::plan_collection_update(
            candidate,
            CollectionSourcePath {
                root,
                database_root,
                relative_path,
            },
            collection,
            command,
        )
    }

    fn apply_collection_updates(
        &mut self,
        collection: SampleCollection,
        updates: Vec<CollectionUpdate>,
        trigger: &'static str,
        command: CollectionCommand,
    ) {
        let started_at = std::time::Instant::now();
        if updates.is_empty() {
            self.ui.status.sample = status::empty_collection_status(command);
            emit_gui_action(
                command.action_name(),
                Some("browser"),
                Some(trigger),
                "empty",
                started_at,
                None,
            );
            return;
        }

        let counts = match state_application::apply_collection_update_states(self, &updates) {
            Ok(counts) => counts,
            Err(error) => {
                self.ui.status.sample = format!("Collection update failed: {error}");
                emit_gui_action(
                    command.action_name(),
                    Some("browser"),
                    Some(trigger),
                    "error",
                    started_at,
                    Some(self.ui.status.sample.as_str()),
                );
                return;
            }
        };

        self.ui.status.sample = status::collection_status(collection, counts, command);
        emit_gui_action(
            command.action_name(),
            Some("browser"),
            Some(trigger),
            "success",
            started_at,
            None,
        );
        if counts.changed() {
            transaction::register_collection_transaction(self, collection, command, updates);
        }
    }

    fn apply_missing_collection_cleanup(
        &mut self,
        collection: SampleCollection,
        files: Vec<MissingCollectionFile>,
        trigger: &'static str,
    ) {
        let started_at = std::time::Instant::now();
        if files.is_empty() {
            self.ui.status.sample = String::from("No missing collection files to clean");
            emit_gui_action(
                "browser.collection.clean_missing",
                Some("browser"),
                Some(trigger),
                "empty",
                started_at,
                None,
            );
            return;
        }

        for ((root, database_root), source_files) in
            persistence::group_missing_collection_files_by_source(&files)
        {
            if let Err(error) = persistence::persist_missing_collection_cleanup(
                &root,
                &database_root,
                &source_files,
            ) {
                self.library
                    .folder_browser
                    .refresh_missing_collection_state();
                self.ui.status.sample = format!("Missing collection cleanup failed: {error}");
                emit_gui_action(
                    "browser.collection.clean_missing",
                    Some("browser"),
                    Some(trigger),
                    "error",
                    started_at,
                    Some(self.ui.status.sample.as_str()),
                );
                return;
            }
        }

        let removed = files.len();
        self.library
            .folder_browser
            .remove_missing_collection_files(&files);
        self.ui.status.sample = format!(
            "Cleaned {} missing sample{} from Collection {}",
            removed,
            if removed == 1 { "" } else { "s" },
            collection_hotkey(collection)
        );
        emit_gui_action(
            "browser.collection.clean_missing",
            Some("browser"),
            Some(trigger),
            "success",
            started_at,
            None,
        );
    }
}
