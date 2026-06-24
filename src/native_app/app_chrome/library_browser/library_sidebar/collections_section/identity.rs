use radiant::prelude as ui;
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::ui::ids as widget_ids;

pub(super) const COLLECTION_ROW_INPUT_SCOPE: u64 = widget_ids::COLLECTION_ROW_INPUT_SCOPE;

pub(super) fn collection_input_id(collection: SampleCollection) -> u64 {
    ui::stable_widget_id_u64(COLLECTION_ROW_INPUT_SCOPE, collection.index() as u64)
}

pub(super) fn collection_row_key(collection: SampleCollection) -> String {
    format!("collection-row-{}", collection.index())
}

pub(super) fn collection_rename_row_key(collection: SampleCollection) -> String {
    format!("collection-rename-row-{}", collection.index())
}

pub(super) fn collection_rename_swatch_key(collection: SampleCollection) -> String {
    format!("collection-rename-swatch-{}", collection.index())
}
