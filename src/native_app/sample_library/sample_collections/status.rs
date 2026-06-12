use wavecrate::sample_sources::SampleCollection;

use super::command::{CollectionCommand, CollectionUpdateCounts};

pub(super) fn empty_collection_status(command: CollectionCommand) -> String {
    match command {
        CollectionCommand::Add | CollectionCommand::Toggle => {
            String::from("Select a sample to add to a collection")
        }
        CollectionCommand::Remove => String::from("Select a collection sample to remove"),
    }
}

pub(super) fn collection_status(
    collection: SampleCollection,
    counts: CollectionUpdateCounts,
    command: CollectionCommand,
) -> String {
    let hotkey =
        crate::native_app::sample_library::folder_browser::view_contract::collection_hotkey(
            collection,
        );
    match command {
        CollectionCommand::Add => format!(
            "Added {} sample{} to Collection {hotkey}",
            counts.added,
            plural(counts.added)
        ),
        CollectionCommand::Remove => format!(
            "Removed {} sample{} from Collection {hotkey}",
            counts.removed,
            plural(counts.removed)
        ),
        CollectionCommand::Toggle => match (counts.added, counts.removed) {
            (0, removed) => {
                format!(
                    "Removed {removed} sample{} from Collection {hotkey}",
                    plural(removed)
                )
            }
            (added, 0) => format!(
                "Added {added} sample{} to Collection {hotkey}",
                plural(added)
            ),
            (added, removed) => {
                format!("Collection {hotkey}: added {added}, removed {removed}")
            }
        },
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
