//! Known metadata tag catalog and category projection helpers.

use std::collections::BTreeSet;

use super::super::vocabulary::{
    DEFAULT_METADATA_TAGS, METADATA_TAG_CATEGORIES, inferred_metadata_tag_category_id,
    metadata_tag_category_is_locked, metadata_tag_category_label_for_id,
    static_metadata_tag_category_id,
};
use super::super::{MetadataTagCategoryGroup, NativeAppState};

impl NativeAppState {
    pub(in crate::native_app::metadata) fn known_metadata_tags(&self) -> Vec<String> {
        DEFAULT_METADATA_TAGS
            .iter()
            .map(|tag| (*tag).to_string())
            .chain(
                self.metadata
                    .tags_by_file
                    .values()
                    .flat_map(|tags| tags.iter().cloned()),
            )
            .chain(self.metadata.tag_dictionary.keys().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(in crate::native_app) fn categorized_metadata_tags(&self) -> Vec<MetadataTagCategoryGroup> {
        let mut groups = METADATA_TAG_CATEGORIES
            .iter()
            .map(|(id, label)| MetadataTagCategoryGroup {
                id,
                label,
                tags: Vec::new(),
                collapsed: self.metadata.collapsed_tag_categories.contains(*id),
                locked: metadata_tag_category_is_locked(id),
            })
            .collect::<Vec<_>>();
        for tag in self.known_metadata_tags() {
            let category_id = self.metadata_tag_category_id(&tag);
            if let Some(group) = groups.iter_mut().find(|group| group.id == category_id) {
                group.tags.push(tag);
            }
        }
        groups
    }

    pub(in crate::native_app::metadata) fn is_new_metadata_tag(&self, tag: &str) -> bool {
        !self.known_metadata_tags().iter().any(|known| known == tag)
    }

    pub(in crate::native_app::metadata) fn metadata_tag_category_id(
        &self,
        tag: &str,
    ) -> &'static str {
        self.metadata
            .tag_dictionary
            .get(tag)
            .and_then(|category_id| {
                metadata_tag_category_label_for_id(category_id).map(|_| category_id.as_str())
            })
            .and_then(static_metadata_tag_category_id)
            .unwrap_or_else(|| inferred_metadata_tag_category_id(tag))
    }

    pub(super) fn metadata_tag_category_label(&self, tag: &str) -> &'static str {
        metadata_tag_category_label_for_id(self.metadata_tag_category_id(tag))
            .unwrap_or("Character")
    }
}
