use wavecrate::sample_sources::SampleCollection;

use crate::native_app::ui::ids as widget_ids;

/// Scope for retained collection-row input identity.
pub(super) const RETAINED_COLLECTION_ROW_INPUT_SCOPE: u64 =
    widget_ids::RETAINED_COLLECTION_ROW_INPUT_SCOPE;

/// Retained row key for a collection row.
pub(super) fn retained_collection_row_key(collection: SampleCollection) -> String {
    format!("collection-row-{}", collection.index())
}

/// Retained row key for a collection rename editor row.
pub(super) fn retained_collection_rename_row_key(collection: SampleCollection) -> String {
    format!("collection-rename-row-{}", collection.index())
}

#[cfg(test)]
/// Retained input id for a collection row.
pub(super) fn retained_collection_row_input_id(collection: SampleCollection) -> u64 {
    radiant::widgets::stable_widget_id(
        RETAINED_COLLECTION_ROW_INPUT_SCOPE,
        retained_collection_row_key(collection),
    )
}

#[cfg(test)]
/// Tests retained collection-row identity helpers.
mod tests {
    use super::*;

    #[test]
    /// Verifies retained collection-row input ids are stable and collection-scoped.
    fn retained_collection_row_input_ids_are_stable_per_collection() {
        let first = SampleCollection::new(0).expect("valid collection");
        let second = SampleCollection::new(1).expect("valid collection");

        assert_eq!(
            retained_collection_row_input_id(first),
            retained_collection_row_input_id(first)
        );
        assert_ne!(
            retained_collection_row_input_id(first),
            retained_collection_row_input_id(second)
        );
    }
}
