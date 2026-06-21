use std::collections::BTreeMap;

use radiant::gui::types::Rgba8;
use wavecrate::sample_sources::SampleCollection;

use super::{
    super::{FolderBrowserDropTarget, FolderBrowserState},
    model::SampleCollectionView,
};

impl FolderBrowserState {
    pub(in crate::native_app) fn visible_collections(&self) -> Vec<SampleCollectionView> {
        let counts = self.collection_counts();
        self.collection_panel
            .collections
            .iter()
            .map(|collection| SampleCollectionView {
                collection: collection.collection,
                hotkey: collection.hotkey,
                name: collection.name.clone(),
                color: collection.color,
                selected: self.selection.selected_collection == Some(collection.collection),
                drop_target: self
                    .drag_drop
                    .drop_target
                    .is_open(&FolderBrowserDropTarget::Collection(collection.collection)),
                drag_active: self.file_drag_active(),
                assigned_count: counts
                    .get(&collection.collection.index())
                    .copied()
                    .unwrap_or_default(),
            })
            .collect()
    }

    pub(in crate::native_app) fn collection_color(
        &self,
        collection: SampleCollection,
    ) -> Option<Rgba8> {
        self.collection_panel
            .collections
            .iter()
            .find(|entry| entry.collection == collection)
            .map(|entry| entry.color)
    }

    pub(super) fn collection_counts(&self) -> BTreeMap<u8, usize> {
        let mut counts = BTreeMap::new();
        for file in self.loaded_source_audio_files() {
            for collection in file.collection_memberships() {
                *counts.entry(collection.index()).or_insert(0) += 1;
            }
        }
        counts
    }
}
