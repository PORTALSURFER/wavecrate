use std::collections::BTreeSet;

use super::vocabulary::{
    DEFAULT_METADATA_TAGS, METADATA_TAG_CATEGORIES, USER_EXTENSIBLE_METADATA_TAG_CATEGORIES,
    inferred_metadata_tag_category_id, metadata_tag_category_is_locked,
    metadata_tag_category_label_for_id, metadata_tag_completions_for_prefix,
    normalize_metadata_category_query, normalize_metadata_tag, static_metadata_tag_category_id,
};
use super::{
    GuiAppState, MetadataTagCategoryGroup, MetadataTagCompletionOption, MetadataTagInputMode,
};

impl GuiAppState {
    pub(in crate::gui_app) fn metadata_tag_completion_suffix(&self) -> Option<String> {
        match &self.metadata_tag_input_mode {
            MetadataTagInputMode::Tag => {
                let prefix = normalize_metadata_tag(&self.metadata_tag_draft)?;
                let suggestion = self.selected_metadata_tag_completion()?;
                if suggestion == prefix {
                    return None;
                }
                suggestion
                    .strip_prefix(prefix.as_str())
                    .map(str::to_string)
                    .filter(|suffix| !suffix.is_empty())
            }
            MetadataTagInputMode::Category { .. } => self.metadata_tag_category_completion_suffix(),
        }
    }

    pub(in crate::gui_app) fn metadata_tag_completion_options(
        &self,
    ) -> Vec<MetadataTagCompletionOption> {
        if matches!(
            self.metadata_tag_input_mode,
            MetadataTagInputMode::Category { .. }
        ) {
            return self.metadata_tag_category_completion_options();
        }
        let suggestions = self.metadata_tag_suggestions();
        let selected_index = self.selected_metadata_tag_completion_index(suggestions.len());
        suggestions
            .into_iter()
            .enumerate()
            .map(|(index, tag)| MetadataTagCompletionOption {
                category: self.metadata_tag_category_label(&tag),
                selected: index == selected_index,
                tag,
            })
            .collect()
    }

    pub(in crate::gui_app) fn move_metadata_tag_completion_selection(&mut self, delta: i32) {
        if matches!(
            self.metadata_tag_input_mode,
            MetadataTagInputMode::Category { .. }
        ) {
            self.move_metadata_tag_category_completion_selection(delta);
            return;
        }
        let Some(prefix) = normalize_metadata_tag(&self.metadata_tag_draft) else {
            self.reset_metadata_tag_completion_cycle();
            return;
        };
        let suggestions = metadata_tag_completions_for_prefix(
            prefix.as_str(),
            self.known_metadata_tags().iter().map(String::as_str),
        );
        if suggestions.is_empty() {
            self.reset_metadata_tag_completion_cycle();
            return;
        }
        let current = if self.metadata_tag_completion_prefix.as_deref() == Some(prefix.as_str()) {
            self.metadata_tag_completion_index % suggestions.len()
        } else {
            0
        };
        self.metadata_tag_completion_prefix = Some(prefix);
        self.metadata_tag_completion_index = radiant::prelude::cyclic_list_index_after_delta(
            current,
            delta as isize,
            suggestions.len(),
        )
        .unwrap_or(0);
    }

    pub(super) fn selected_metadata_tag_completion(&self) -> Option<String> {
        let suggestions = self.metadata_tag_suggestions();
        let index = self.selected_metadata_tag_completion_index(suggestions.len());
        suggestions.get(index).cloned()
    }

    fn metadata_tag_suggestions(&self) -> Vec<String> {
        let Some(prefix) = normalize_metadata_tag(&self.metadata_tag_draft) else {
            return Vec::new();
        };
        metadata_tag_completions_for_prefix(
            prefix.as_str(),
            self.known_metadata_tags().iter().map(String::as_str),
        )
    }

    pub(in crate::gui_app) fn metadata_tag_completion_active(&self) -> bool {
        if matches!(
            self.metadata_tag_input_mode,
            MetadataTagInputMode::Category { .. }
        ) {
            return !self.metadata_tag_category_suggestions().is_empty();
        }
        !self.metadata_tag_suggestions().is_empty()
    }

    fn selected_metadata_tag_completion_index(&self, suggestion_count: usize) -> usize {
        if suggestion_count == 0 {
            return 0;
        }
        let Some(prefix) = normalize_metadata_tag(&self.metadata_tag_draft) else {
            return 0;
        };
        if self.metadata_tag_completion_prefix.as_deref() == Some(prefix.as_str()) {
            self.metadata_tag_completion_index % suggestion_count
        } else {
            0
        }
    }

    pub(super) fn known_metadata_tags(&self) -> Vec<String> {
        DEFAULT_METADATA_TAGS
            .iter()
            .map(|tag| (*tag).to_string())
            .chain(
                self.metadata_tags_by_file
                    .values()
                    .flat_map(|tags| tags.iter().cloned()),
            )
            .chain(self.metadata_tag_dictionary.keys().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(in crate::gui_app) fn categorized_metadata_tags(&self) -> Vec<MetadataTagCategoryGroup> {
        let mut groups = METADATA_TAG_CATEGORIES
            .iter()
            .map(|(id, label)| MetadataTagCategoryGroup {
                id,
                label,
                tags: Vec::new(),
                collapsed: self.collapsed_metadata_tag_categories.contains(*id),
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

    pub(super) fn is_new_metadata_tag(&self, tag: &str) -> bool {
        !self.known_metadata_tags().iter().any(|known| known == tag)
    }

    pub(super) fn metadata_tag_category_id(&self, tag: &str) -> &'static str {
        self.metadata_tag_dictionary
            .get(tag)
            .and_then(|category_id| {
                metadata_tag_category_label_for_id(category_id).map(|_| category_id.as_str())
            })
            .and_then(static_metadata_tag_category_id)
            .unwrap_or_else(|| inferred_metadata_tag_category_id(tag))
    }

    fn metadata_tag_category_label(&self, tag: &str) -> &'static str {
        metadata_tag_category_label_for_id(self.metadata_tag_category_id(tag))
            .unwrap_or("Character")
    }

    fn metadata_tag_category_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        let Some(prefix) = normalize_metadata_category_query(&self.metadata_tag_draft) else {
            return Vec::new();
        };
        USER_EXTENSIBLE_METADATA_TAG_CATEGORIES
            .into_iter()
            .filter(|(id, label)| {
                normalize_metadata_category_query(id)
                    .is_some_and(|normalized| normalized.starts_with(prefix.as_str()))
                    || normalize_metadata_category_query(label)
                        .is_some_and(|normalized| normalized.starts_with(prefix.as_str()))
            })
            .collect()
    }

    pub(super) fn selected_metadata_tag_category(&self, value: &str) -> Option<&'static str> {
        let suggestions = self.metadata_tag_category_suggestions();
        if !suggestions.is_empty() {
            let index = self.selected_metadata_tag_completion_index(suggestions.len());
            return suggestions.get(index).map(|(id, _)| *id);
        }
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

    fn metadata_tag_category_completion_options(&self) -> Vec<MetadataTagCompletionOption> {
        let suggestions = self.metadata_tag_category_suggestions();
        let selected_index = self.selected_metadata_tag_completion_index(suggestions.len());
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

    fn metadata_tag_category_completion_suffix(&self) -> Option<String> {
        let prefix = normalize_metadata_category_query(&self.metadata_tag_draft)?;
        let suggestions = self.metadata_tag_category_suggestions();
        let index = self.selected_metadata_tag_completion_index(suggestions.len());
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

    fn move_metadata_tag_category_completion_selection(&mut self, delta: i32) {
        let Some(prefix) = normalize_metadata_category_query(&self.metadata_tag_draft) else {
            self.reset_metadata_tag_completion_cycle();
            return;
        };
        let suggestions = self.metadata_tag_category_suggestions();
        if suggestions.is_empty() {
            self.reset_metadata_tag_completion_cycle();
            return;
        }
        let current = if self.metadata_tag_completion_prefix.as_deref() == Some(prefix.as_str()) {
            self.metadata_tag_completion_index % suggestions.len()
        } else {
            0
        };
        self.metadata_tag_completion_prefix = Some(prefix);
        self.metadata_tag_completion_index = radiant::prelude::cyclic_list_index_after_delta(
            current,
            delta as isize,
            suggestions.len(),
        )
        .unwrap_or(0);
    }

    pub(super) fn reset_metadata_tag_completion_cycle(&mut self) {
        self.metadata_tag_completion_prefix = None;
        self.metadata_tag_completion_index = 0;
    }
}
