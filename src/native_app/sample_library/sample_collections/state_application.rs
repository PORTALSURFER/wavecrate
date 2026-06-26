use crate::native_app::app::NativeAppState;

use super::{
    command::{CollectionOperation, CollectionUpdate, CollectionUpdateCounts},
    persistence::{group_updates_by_source, persist_collection_updates},
};

pub(super) fn apply_collection_update_states(
    state: &mut NativeAppState,
    updates: &[CollectionUpdate],
) -> Result<CollectionUpdateCounts, String> {
    let mut counts = CollectionUpdateCounts::default();
    for ((root, database_root), source_updates) in group_updates_by_source(updates) {
        persist_collection_updates(&root, &database_root, &source_updates)?;
        for update in source_updates {
            match update.operation {
                CollectionOperation::Add => {
                    if state
                        .library
                        .folder_browser
                        .set_file_collection_state(&update.absolute_path, update.collection)
                    {
                        counts.added += 1;
                    }
                }
                CollectionOperation::Remove => {
                    if state
                        .library
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
