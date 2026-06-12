//! Tag value completion and selection behavior.

use super::super::vocabulary::{metadata_tag_completions_for_prefix, normalize_metadata_tag};
use super::super::{MetadataTagCompletionOption, NativeAppState};

impl NativeAppState {
    pub(super) fn metadata_tag_value_completion_suffix(&self) -> Option<String> {
        let prefix = normalize_metadata_tag(&self.metadata.tag_draft)?;
        let suggestion = self.first_metadata_tag_completion()?;
        if suggestion == prefix {
            return None;
        }
        suggestion
            .strip_prefix(prefix.as_str())
            .map(str::to_string)
            .filter(|suffix| !suffix.is_empty())
    }

    pub(super) fn metadata_tag_value_completion_options(&self) -> Vec<MetadataTagCompletionOption> {
        let suggestions = self.metadata_tag_suggestions();
        let selected_index = self.explicit_metadata_tag_completion_index(suggestions.len());
        suggestions
            .into_iter()
            .enumerate()
            .map(|(index, tag)| MetadataTagCompletionOption {
                category: self.metadata_tag_category_label(&tag),
                selected: selected_index == Some(index),
                tag,
            })
            .collect()
    }

    pub(super) fn move_metadata_tag_value_completion_selection(&mut self, delta: i32) {
        self.metadata.pending_tag_completion_query = None;
        let Some(prefix) = normalize_metadata_tag(&self.metadata.tag_draft) else {
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
        self.metadata.tag_completion_cycle.move_selection_from_edge(
            prefix,
            delta as isize,
            suggestions.len(),
        );
    }

    pub(super) fn hover_metadata_tag_value_completion(&mut self, value: String) {
        let Some(prefix) = normalize_metadata_tag(&self.metadata.tag_draft) else {
            self.reset_metadata_tag_completion_cycle();
            return;
        };
        let suggestions = self.metadata_tag_suggestions();
        let Some(index) = suggestions.iter().position(|tag| tag == &value) else {
            return;
        };
        self.metadata
            .tag_completion_cycle
            .select(prefix, index, suggestions.len());
    }

    pub(super) fn activate_metadata_tag_value_completion(&mut self) {
        let Some(prefix) = normalize_metadata_tag(&self.metadata.tag_draft) else {
            self.reset_metadata_tag_completion_cycle();
            return;
        };
        let suggestion_count = self.metadata_tag_suggestions().len();
        if suggestion_count == 0 {
            self.reset_metadata_tag_completion_cycle();
            return;
        }
        if self.metadata.tag_completion_cycle.query_key() == Some(prefix.as_str()) {
            self.metadata.pending_tag_completion_query = None;
            return;
        }
        if self.metadata.pending_tag_completion_query.as_deref() != Some(prefix.as_str()) {
            self.metadata.tag_completion_cycle.reset();
            self.metadata.pending_tag_completion_query = Some(prefix);
            return;
        }
        self.metadata.pending_tag_completion_query = None;
        self.metadata
            .tag_completion_cycle
            .select(prefix, 0, suggestion_count);
    }

    pub(super) fn explicit_metadata_tag_value_completion(&self) -> Option<String> {
        let suggestions = self.metadata_tag_suggestions();
        let index = self.explicit_metadata_tag_completion_index(suggestions.len())?;
        suggestions.get(index).cloned()
    }

    pub(super) fn metadata_tag_value_completion_active(&self) -> bool {
        !self.metadata_tag_suggestions().is_empty()
    }

    fn first_metadata_tag_completion(&self) -> Option<String> {
        self.metadata_tag_suggestions().into_iter().next()
    }

    fn metadata_tag_suggestions(&self) -> Vec<String> {
        let Some(prefix) = normalize_metadata_tag(&self.metadata.tag_draft) else {
            return Vec::new();
        };
        metadata_tag_completions_for_prefix(
            prefix.as_str(),
            self.known_metadata_tags().iter().map(String::as_str),
        )
    }

    fn explicit_metadata_tag_completion_index(&self, suggestion_count: usize) -> Option<usize> {
        if suggestion_count == 0 {
            return None;
        }
        let prefix = normalize_metadata_tag(&self.metadata.tag_draft)?;
        self.metadata
            .tag_completion_cycle
            .active_selected_index(prefix.as_str(), suggestion_count)
    }
}
