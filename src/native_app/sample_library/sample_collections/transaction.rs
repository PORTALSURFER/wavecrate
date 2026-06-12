use wavecrate::sample_sources::SampleCollection;

use crate::native_app::{app::NativeAppState, transaction_history::TransactionContext};

use super::{
    command::{CollectionCommand, CollectionUpdate, CollectionUpdateCounts},
    state_application,
};

pub(super) fn register_collection_transaction(
    state: &mut NativeAppState,
    collection: SampleCollection,
    command: CollectionCommand,
    updates: Vec<CollectionUpdate>,
) {
    let hotkey =
        crate::native_app::sample_library::folder_browser::view_contract::collection_hotkey(
            collection,
        );
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
    state.begin_transaction(label);
    state.register_transaction_action(
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
    state.commit_transaction();
}

impl TransactionContext<'_> {
    fn apply_collection_update_states(
        &mut self,
        updates: &[CollectionUpdate],
    ) -> Result<CollectionUpdateCounts, String> {
        state_application::apply_collection_update_states(self.state, updates)
    }
}
