use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use wavecrate::sample_sources::{SampleCollection, SourceDatabase};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::transaction_history::TransactionContext;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CollectionUpdate {
    root: PathBuf,
    relative_path: PathBuf,
    absolute_path: PathBuf,
    collection: SampleCollection,
    operation: CollectionOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionOperation {
    Add,
    Remove,
}

impl CollectionOperation {
    fn inverted(self) -> Self {
        match self {
            Self::Add => Self::Remove,
            Self::Remove => Self::Add,
        }
    }
}

impl CollectionUpdate {
    fn inverted(mut self) -> Self {
        self.operation = self.operation.inverted();
        self
    }
}

#[derive(Default)]
struct CollectionUpdateCounts {
    added: usize,
    removed: usize,
}

impl NativeAppState {
    pub(in crate::native_app) fn assign_selected_collection(
        &mut self,
        collection: SampleCollection,
        _context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let updates = self.collection_updates_for_selected_files(collection);
        self.apply_collection_updates(collection, updates, "hotkey", CollectionCommand::Toggle);
    }

    pub(in crate::native_app) fn remove_context_sample_from_collection(
        &mut self,
        _context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let Some(menu) = self.browser_interaction.context_menu.take() else {
            return;
        };
        let Some(collection) = menu.collection else {
            self.sample_status = String::from("Sample is not in the active collection");
            return;
        };
        let updates = self
            .folder_browser
            .context_file_collection_candidate(&menu.path, collection)
            .and_then(|candidate| {
                collection_update_for_candidate(
                    self,
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

    pub(in crate::native_app) fn drop_drag_on_collection(
        &mut self,
        collection: SampleCollection,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let updates = self.collection_updates_for_dragged_files(collection);
        context.end_drag_session();
        self.folder_browser.clear_drag();
        self.apply_collection_updates(collection, updates, "drop", CollectionCommand::Add);
    }

    fn collection_updates_for_selected_files(
        &self,
        collection: SampleCollection,
    ) -> Vec<CollectionUpdate> {
        self.folder_browser
            .selected_file_collection_candidates(collection)
            .into_iter()
            .filter_map(|candidate| {
                collection_update_for_candidate(
                    self,
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
        self.folder_browser
            .drag_file_collection_candidates(collection)
            .into_iter()
            .filter_map(|candidate| {
                collection_update_for_candidate(self, candidate, collection, CollectionCommand::Add)
            })
            .collect()
    }

    fn apply_collection_updates(
        &mut self,
        collection: SampleCollection,
        updates: Vec<CollectionUpdate>,
        trigger: &'static str,
        command: CollectionCommand,
    ) {
        let started_at = Instant::now();
        if updates.is_empty() {
            self.sample_status = match command {
                CollectionCommand::Add | CollectionCommand::Toggle => {
                    String::from("Select a sample to add to a collection")
                }
                CollectionCommand::Remove => String::from("Select a collection sample to remove"),
            };
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

        let counts = match self.apply_collection_update_states(&updates) {
            Ok(counts) => counts,
            Err(error) => {
                self.sample_status = format!("Collection update failed: {error}");
                emit_gui_action(
                    command.action_name(),
                    Some("browser"),
                    Some(trigger),
                    "error",
                    started_at,
                    Some(self.sample_status.as_str()),
                );
                return;
            }
        };

        self.sample_status = collection_status(collection, counts.added, counts.removed, command);
        emit_gui_action(
            command.action_name(),
            Some("browser"),
            Some(trigger),
            "success",
            started_at,
            None,
        );
        if counts.added > 0 || counts.removed > 0 {
            self.register_collection_transaction(collection, command, updates);
        }
    }

    fn register_collection_transaction(
        &mut self,
        collection: SampleCollection,
        command: CollectionCommand,
        updates: Vec<CollectionUpdate>,
    ) {
        let hotkey =
            crate::native_app::sample_library::folder_browser::collection_hotkey(collection);
        let label = match command {
            CollectionCommand::Add => format!("Add to Collection {hotkey}"),
            CollectionCommand::Remove => format!("Remove from Collection {hotkey}"),
            CollectionCommand::Toggle => format!("Toggle Collection {hotkey}"),
        };
        let undo_updates = updates
            .iter()
            .cloned()
            .map(CollectionUpdate::inverted)
            .collect::<Vec<_>>();
        let redo_updates = updates;
        self.begin_transaction(label);
        self.register_transaction_action(
            "Apply collection changes",
            move |transaction| {
                transaction
                    .apply_collection_update_states(&undo_updates)
                    .map(|_| ())
            },
            move |transaction| {
                transaction
                    .apply_collection_update_states(&redo_updates)
                    .map(|_| ())
            },
        );
        self.commit_transaction();
    }

    fn apply_collection_update_states(
        &mut self,
        updates: &[CollectionUpdate],
    ) -> Result<CollectionUpdateCounts, String> {
        let mut counts = CollectionUpdateCounts::default();
        for (root, source_updates) in group_updates_by_source(updates.to_vec()) {
            persist_collection_updates(&root, &source_updates)?;
            for update in source_updates {
                match update.operation {
                    CollectionOperation::Add => {
                        if self
                            .folder_browser
                            .set_file_collection_state(&update.absolute_path, update.collection)
                        {
                            counts.added += 1;
                        }
                    }
                    CollectionOperation::Remove => {
                        if self
                            .folder_browser
                            .remove_file_collection_state(&update.absolute_path, update.collection)
                        {
                            counts.removed += 1;
                        }
                    }
                }
            }
        }
        Ok(counts)
    }
}

impl TransactionContext<'_> {
    fn apply_collection_update_states(
        &mut self,
        updates: &[CollectionUpdate],
    ) -> Result<CollectionUpdateCounts, String> {
        self.state.apply_collection_update_states(updates)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionCommand {
    Add,
    Remove,
    Toggle,
}

impl CollectionCommand {
    fn action_name(self) -> &'static str {
        match self {
            Self::Add => "browser.collection.assign",
            Self::Remove => "browser.collection.remove",
            Self::Toggle => "browser.collection.toggle",
        }
    }
}

fn collection_update_for_candidate(
    state: &NativeAppState,
    candidate: crate::native_app::sample_library::folder_browser::SelectedFileCollectionCandidate,
    collection: SampleCollection,
    command: CollectionCommand,
) -> Option<CollectionUpdate> {
    let operation = match command {
        CollectionCommand::Add => {
            if candidate.assigned {
                return None;
            }
            CollectionOperation::Add
        }
        CollectionCommand::Remove => {
            if !candidate.assigned {
                return None;
            }
            CollectionOperation::Remove
        }
        CollectionCommand::Toggle => {
            if candidate.assigned {
                CollectionOperation::Remove
            } else {
                CollectionOperation::Add
            }
        }
    };
    let (root, relative_path) = state
        .folder_browser
        .source_relative_file_path(&candidate.path)?;
    Some(CollectionUpdate {
        root,
        relative_path,
        absolute_path: candidate.path,
        collection,
        operation,
    })
}

fn group_updates_by_source(
    updates: Vec<CollectionUpdate>,
) -> BTreeMap<PathBuf, Vec<CollectionUpdate>> {
    let mut by_source: BTreeMap<PathBuf, Vec<CollectionUpdate>> = BTreeMap::new();
    for update in updates {
        by_source
            .entry(update.root.clone())
            .or_default()
            .push(update);
    }
    by_source
}

fn persist_collection_updates(root: &Path, updates: &[CollectionUpdate]) -> Result<(), String> {
    let db = SourceDatabase::open_for_user_metadata_write(root).map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    for update in updates {
        let (file_size, modified_ns) = file_metadata(&update.absolute_path)?;
        batch
            .upsert_file(&update.relative_path, file_size, modified_ns)
            .map_err(|err| err.to_string())?;
        match update.operation {
            CollectionOperation::Add => batch
                .add_collection(&update.relative_path, update.collection)
                .map_err(|err| err.to_string())?,
            CollectionOperation::Remove => batch
                .remove_collection(&update.relative_path, update.collection)
                .map_err(|err| err.to_string())?,
        }
    }
    batch.commit().map_err(|err| err.to_string())
}

fn collection_status(
    collection: SampleCollection,
    added: usize,
    removed: usize,
    command: CollectionCommand,
) -> String {
    let hotkey = crate::native_app::sample_library::folder_browser::collection_hotkey(collection);
    match command {
        CollectionCommand::Add => format!(
            "Added {added} sample{} to Collection {hotkey}",
            if added == 1 { "" } else { "s" }
        ),
        CollectionCommand::Remove => format!(
            "Removed {removed} sample{} from Collection {hotkey}",
            if removed == 1 { "" } else { "s" }
        ),
        CollectionCommand::Toggle => match (added, removed) {
            (0, removed) => format!(
                "Removed {removed} sample{} from Collection {hotkey}",
                if removed == 1 { "" } else { "s" }
            ),
            (added, 0) => format!(
                "Added {added} sample{} to Collection {hotkey}",
                if added == 1 { "" } else { "s" }
            ),
            (added, removed) => {
                format!("Collection {hotkey}: added {added}, removed {removed}")
            }
        },
    }
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| String::from("File modified time is before epoch"))?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}
