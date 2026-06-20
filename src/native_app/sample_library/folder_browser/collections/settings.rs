use std::collections::BTreeMap;

use wavecrate::sample_sources::SampleCollection;

use super::{CollectionPanelState, default_collections};
use crate::native_app::sample_library::folder_browser::FolderBrowserState;

impl FolderBrowserState {
    pub(in crate::native_app) fn apply_collection_names(
        &mut self,
        names: &BTreeMap<String, String>,
    ) {
        self.collection_panel.apply_collection_names(names);
    }

    pub(in crate::native_app) fn custom_collection_names(&self) -> BTreeMap<String, String> {
        self.collection_panel.custom_collection_names()
    }
}

impl CollectionPanelState {
    fn apply_collection_names(&mut self, names: &BTreeMap<String, String>) {
        for (key, value) in names {
            let Some(collection) = collection_from_settings_key(key) else {
                continue;
            };
            let label = value.trim();
            if label.is_empty() {
                continue;
            }
            if let Some(entry) = self
                .collections
                .iter_mut()
                .find(|entry| entry.collection == collection)
            {
                entry.name = label.to_string();
            }
        }
    }

    fn custom_collection_names(&self) -> BTreeMap<String, String> {
        let defaults = default_collections()
            .into_iter()
            .map(|entry| (entry.collection.index(), entry.name))
            .collect::<BTreeMap<_, _>>();

        self.collections
            .iter()
            .filter_map(|entry| {
                let label = entry.name.trim();
                let default = defaults.get(&entry.collection.index()).map(String::as_str);
                (!label.is_empty() && Some(label) != default)
                    .then(|| (collection_settings_key(entry.collection), label.to_string()))
            })
            .collect()
    }
}

fn collection_from_settings_key(key: &str) -> Option<SampleCollection> {
    key.parse::<u8>().ok().and_then(SampleCollection::new)
}

fn collection_settings_key(collection: SampleCollection) -> String {
    collection.index().to_string()
}
