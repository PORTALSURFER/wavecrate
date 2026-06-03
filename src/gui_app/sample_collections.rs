use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use wavecrate::sample_sources::{SampleCollection, SourceDatabase};

use super::{GuiAppState, GuiMessage, emit_gui_action};

#[derive(Clone, Debug, PartialEq, Eq)]
struct CollectionUpdate {
    root: PathBuf,
    relative_path: PathBuf,
    absolute_path: PathBuf,
    collection: SampleCollection,
}

impl GuiAppState {
    pub(super) fn assign_selected_collection(
        &mut self,
        collection: SampleCollection,
        _context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let updates = self.collection_updates_for_selected_files(collection);
        self.apply_collection_updates(collection, updates, "hotkey");
    }

    pub(super) fn drop_drag_on_collection(
        &mut self,
        collection: SampleCollection,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let updates = self.collection_updates_for_dragged_files(collection);
        context.end_drag_session();
        self.folder_browser.clear_drag();
        self.apply_collection_updates(collection, updates, "drop");
    }

    fn collection_updates_for_selected_files(
        &self,
        collection: SampleCollection,
    ) -> Vec<CollectionUpdate> {
        self.folder_browser
            .selected_file_collection_candidates()
            .into_iter()
            .filter_map(|candidate| {
                collection_update_for_candidate(self, candidate.path, collection)
            })
            .collect()
    }

    fn collection_updates_for_dragged_files(
        &self,
        collection: SampleCollection,
    ) -> Vec<CollectionUpdate> {
        self.folder_browser
            .drag_file_collection_candidates()
            .into_iter()
            .filter_map(|candidate| {
                collection_update_for_candidate(self, candidate.path, collection)
            })
            .collect()
    }

    fn apply_collection_updates(
        &mut self,
        collection: SampleCollection,
        updates: Vec<CollectionUpdate>,
        trigger: &'static str,
    ) {
        let started_at = Instant::now();
        if updates.is_empty() {
            self.sample_status = String::from("Select a sample to add to a collection");
            emit_gui_action(
                "browser.collection.assign",
                Some("browser"),
                Some(trigger),
                "empty",
                started_at,
                None,
            );
            return;
        }

        let mut applied = 0usize;
        let mut last_error = None;
        for (root, source_updates) in group_updates_by_source(updates) {
            match persist_collection_updates(&root, &source_updates) {
                Ok(()) => {
                    for update in source_updates {
                        if self
                            .folder_browser
                            .set_file_collection_state(&update.absolute_path, collection)
                        {
                            applied += 1;
                        }
                    }
                }
                Err(error) => last_error = Some(error),
            }
        }

        if let Some(error) = last_error {
            self.sample_status = format!("Collection update failed: {error}");
            emit_gui_action(
                "browser.collection.assign",
                Some("browser"),
                Some(trigger),
                "error",
                started_at,
                Some(self.sample_status.as_str()),
            );
            return;
        }

        self.sample_status = format!(
            "Added {applied} sample{} to Collection {}",
            if applied == 1 { "" } else { "s" },
            crate::gui_app::folder_browser::collection_hotkey(collection)
        );
        emit_gui_action(
            "browser.collection.assign",
            Some("browser"),
            Some(trigger),
            "success",
            started_at,
            None,
        );
    }
}

fn collection_update_for_candidate(
    state: &GuiAppState,
    path: PathBuf,
    collection: SampleCollection,
) -> Option<CollectionUpdate> {
    let (root, relative_path) = state.folder_browser.source_relative_file_path(&path)?;
    Some(CollectionUpdate {
        root,
        relative_path,
        absolute_path: path,
        collection,
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
        batch
            .add_collection(&update.relative_path, update.collection)
            .map_err(|err| err.to_string())?;
    }
    batch.commit().map_err(|err| err.to_string())
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
