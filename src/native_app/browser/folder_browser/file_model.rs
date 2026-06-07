use serde::{Deserialize, Serialize};
use wavecrate::sample_sources::{Rating, SampleCollection};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::native_app) struct FileEntry {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) name: String,
    pub(in crate::native_app) stem: String,
    pub(in crate::native_app) extension: String,
    pub(in crate::native_app) kind: String,
    pub(in crate::native_app) size: String,
    pub(in crate::native_app) size_bytes: u64,
    pub(in crate::native_app) modified: String,
    pub(in crate::native_app) modified_rank: u64,
    pub(in crate::native_app) rating: Rating,
    pub(in crate::native_app) rating_locked: bool,
    pub(in crate::native_app) collection: Option<SampleCollection>,
    #[serde(default)]
    pub(in crate::native_app) collections: Vec<SampleCollection>,
}

impl FileEntry {
    pub(super) fn name_sort_key(&self) -> String {
        self.name.to_ascii_lowercase()
    }

    pub(super) fn is_audio(&self) -> bool {
        self.kind == "Audio"
    }

    pub(in crate::native_app) fn belongs_to_collection(
        &self,
        collection: SampleCollection,
    ) -> bool {
        self.collection == Some(collection) || self.collections.contains(&collection)
    }

    pub(in crate::native_app) fn add_collection(&mut self, collection: SampleCollection) -> bool {
        if self.belongs_to_collection(collection) {
            return false;
        }
        if self.collection.is_none() {
            self.collection = Some(collection);
        }
        self.collections.push(collection);
        self.collections
            .sort_by_key(|collection| collection.index());
        true
    }

    pub(in crate::native_app) fn remove_collection(
        &mut self,
        collection: SampleCollection,
    ) -> bool {
        if !self.belongs_to_collection(collection) {
            return false;
        }
        self.collections.retain(|entry| *entry != collection);
        if self.collection == Some(collection) {
            self.collection = self.collections.first().copied();
        }
        true
    }

    pub(in crate::native_app) fn first_collection(&self) -> Option<SampleCollection> {
        self.collections.first().copied().or(self.collection)
    }

    pub(in crate::native_app) fn collection_memberships(&self) -> Vec<SampleCollection> {
        let mut collections = self.collections.clone();
        if let Some(collection) = self.collection
            && !collections.contains(&collection)
        {
            collections.push(collection);
        }
        collections.sort_by_key(|collection| collection.index());
        collections
    }
}

pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
