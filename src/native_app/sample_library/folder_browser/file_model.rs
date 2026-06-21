use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use wavecrate::sample_sources::{Rating, SampleCollection};

const AUDIO_KIND: &str = "Audio";
const MISSING_KIND: &str = "Missing";

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
        self.kind == AUDIO_KIND || self.is_missing()
    }

    pub(in crate::native_app) fn is_missing(&self) -> bool {
        self.kind == MISSING_KIND
    }

    pub(in crate::native_app) fn missing_collection_member(
        path: &Path,
        rating: Rating,
        rating_locked: bool,
        collections: Vec<SampleCollection>,
        last_played_at: Option<i64>,
    ) -> Self {
        Self {
            id: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string()),
            stem: path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .filter(|stem| !stem.is_empty())
                .unwrap_or_else(|| path.display().to_string()),
            extension: path
                .extension()
                .map(|extension| extension.to_string_lossy().to_string())
                .unwrap_or_default(),
            kind: String::from(MISSING_KIND),
            size: String::from("Missing"),
            size_bytes: 0,
            modified: last_played_label(last_played_at),
            modified_rank: last_played_rank(last_played_at),
            rating,
            rating_locked,
            collection: collections.first().copied(),
            collections,
        }
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

    pub(in crate::native_app) fn set_last_played_at(&mut self, last_played_at: Option<i64>) {
        self.modified = last_played_label(last_played_at);
        self.modified_rank = last_played_rank(last_played_at);
    }
}

pub(super) fn last_played_label(last_played_at: Option<i64>) -> String {
    let Some(last_played_at) = last_played_at else {
        return String::from("Never");
    };
    let age = playback_age(last_played_at);
    let days = age.as_secs() / 86_400;
    if days == 0 {
        String::from("Today")
    } else if days == 1 {
        String::from("1 day")
    } else {
        format!("{days} days")
    }
}

pub(super) fn last_played_rank(last_played_at: Option<i64>) -> u64 {
    last_played_at
        .map(playback_age)
        .map(|age| age.as_secs())
        .unwrap_or(u64::MAX)
}

fn playback_age(last_played_at: i64) -> Duration {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let played = u64::try_from(last_played_at).unwrap_or_default();
    Duration::from_secs(now.saturating_sub(played))
}

pub(in crate::native_app) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
