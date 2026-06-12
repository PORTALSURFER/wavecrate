//! Category completion and pending-tag category resolution.

use super::super::vocabulary::{
    USER_EXTENSIBLE_METADATA_TAG_CATEGORIES, normalize_metadata_category_query,
};
use super::super::{MetadataTagCompletionOption, NativeAppState};

impl NativeAppState {
    pub(super) fn metadata_tag_category_completion_active(&self) -> bool {
        !self.metadata_tag_category_suggestions().is_empty()
    }

    pub(in crate::native_app::metadata) fn selected_metadata_tag_category(
        &self,
        value: &str,
    ) -> Option<&'static str> {
        let suggestions = self.metadata_tag_category_suggestions();
        if !suggestions.is_empty() {
            let index = self.selected_metadata_tag_category_completion_index(suggestions.len());
            return suggestions.get(index).map(|(id, _)| *id);
        }
        self.metadata_tag_category_for_value(value)
    }

    pub(in crate::native_app::metadata) fn metadata_tag_category_for_value(
        &self,
        value: &str,
    ) -> Option<&'static str> {
        let normalized = normalize_metadata_category_query(value)?;
        USER_EXTENSIBLE_METADATA_TAG_CATEGORIES
            .into_iter()
            .find(|(id, label)| {
                normalize_metadata_category_query(id).as_deref() == Some(normalized.as_str())
                    || normalize_metadata_category_query(label).as_deref()
                        == Some(normalized.as_str())
            })
            .map(|(id, _)| id)
    }

    pub(super) fn metadata_tag_category_completion_options(
        &self,
    ) -> Vec<MetadataTagCompletionOption> {
        let suggestions = self.metadata_tag_category_suggestions();
        let selected_index =
            self.selected_metadata_tag_category_completion_index(suggestions.len());
        suggestions
            .into_iter()
            .enumerate()
            .map(|(index, (_id, label))| MetadataTagCompletionOption {
                tag: label.to_string(),
                category: "Group",
                selected: index == selected_index,
            })
            .collect()
    }

    pub(super) fn metadata_tag_category_completion_suffix(&self) -> Option<String> {
        let prefix = normalize_metadata_category_query(&self.metadata.tag_draft)?;
        let suggestions = self.metadata_tag_category_suggestions();
        let index = self.selected_metadata_tag_category_completion_index(suggestions.len());
        let (_id, label) = suggestions.get(index)?;
        let normalized_label = normalize_metadata_category_query(label)?;
        if normalized_label == prefix {
            return None;
        }
        normalized_label
            .strip_prefix(prefix.as_str())
            .map(str::to_string)
            .filter(|suffix| !suffix.is_empty())
    }

    pub(super) fn move_metadata_tag_category_completion_selection(&mut self, delta: i32) {
        let Some(prefix) = self.metadata_tag_category_query_key() else {
            self.reset_metadata_tag_completion_cycle();
            return;
        };
        let suggestions = self.metadata_tag_category_suggestions();
        if suggestions.is_empty() {
            self.reset_metadata_tag_completion_cycle();
            return;
        }
        self.metadata.tag_completion_cycle.move_selection(
            prefix,
            delta as isize,
            suggestions.len(),
        );
    }

    pub(super) fn hover_metadata_tag_category_completion(&mut self, value: String) {
        let Some(prefix) = self.metadata_tag_category_query_key() else {
            self.reset_metadata_tag_completion_cycle();
            return;
        };
        let suggestions = self.metadata_tag_category_suggestions();
        let Some(index) = suggestions
            .iter()
            .position(|(id, label)| id == &value || label == &value)
        else {
            return;
        };
        self.metadata
            .tag_completion_cycle
            .select(prefix, index, suggestions.len());
    }

    fn metadata_tag_category_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        let Some(prefix) = self.metadata_tag_category_query_key() else {
            return Vec::new();
        };
        USER_EXTENSIBLE_METADATA_TAG_CATEGORIES
            .into_iter()
            .filter(|(id, label)| {
                if prefix.is_empty() {
                    return true;
                }
                normalize_metadata_category_query(id)
                    .is_some_and(|normalized| normalized.starts_with(prefix.as_str()))
                    || normalize_metadata_category_query(label)
                        .is_some_and(|normalized| normalized.starts_with(prefix.as_str()))
            })
            .collect()
    }

    fn selected_metadata_tag_category_completion_index(&self, suggestion_count: usize) -> usize {
        if suggestion_count == 0 {
            return 0;
        }
        let Some(prefix) = self.metadata_tag_category_query_key() else {
            return 0;
        };
        self.metadata
            .tag_completion_cycle
            .selected_index(prefix.as_str(), suggestion_count)
            .unwrap_or(0)
    }

    fn metadata_tag_category_query_key(&self) -> Option<String> {
        if self.metadata.tag_draft.trim().is_empty() {
            return Some(String::new());
        }
        normalize_metadata_category_query(&self.metadata.tag_draft)
    }
}
