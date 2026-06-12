//! Metadata tag completion facade.

/// Known-tag catalog and categorized projection helpers.
mod catalog;
/// Pending tag category completion helpers.
mod category;
/// Tag value completion helpers.
mod tag_value;

use super::{MetadataTagInputMode, NativeAppState};

impl NativeAppState {
    pub(in crate::native_app) fn metadata_tag_completion_suffix(&self) -> Option<String> {
        match &self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => self.metadata_tag_value_completion_suffix(),
            MetadataTagInputMode::Category { .. } => self.metadata_tag_category_completion_suffix(),
        }
    }

    pub(in crate::native_app) fn metadata_tag_completion_options(
        &self,
    ) -> Vec<super::MetadataTagCompletionOption> {
        match self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => self.metadata_tag_value_completion_options(),
            MetadataTagInputMode::Category { .. } => {
                self.metadata_tag_category_completion_options()
            }
        }
    }

    pub(in crate::native_app) fn move_metadata_tag_completion_selection(&mut self, delta: i32) {
        match self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => self.move_metadata_tag_value_completion_selection(delta),
            MetadataTagInputMode::Category { .. } => {
                self.move_metadata_tag_category_completion_selection(delta);
            }
        }
    }

    pub(in crate::native_app) fn hover_metadata_tag_completion(&mut self, value: String) {
        match self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => self.hover_metadata_tag_value_completion(value),
            MetadataTagInputMode::Category { .. } => {
                self.hover_metadata_tag_category_completion(value);
            }
        }
    }

    pub(super) fn activate_metadata_tag_completion(&mut self) {
        match self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => self.activate_metadata_tag_value_completion(),
            MetadataTagInputMode::Category { .. } => {
                self.move_metadata_tag_category_completion_selection(0);
            }
        }
    }

    pub(super) fn explicit_metadata_tag_completion(&self) -> Option<String> {
        self.explicit_metadata_tag_value_completion()
    }

    pub(in crate::native_app) fn metadata_tag_completion_active(&self) -> bool {
        match self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => self.metadata_tag_value_completion_active(),
            MetadataTagInputMode::Category { .. } => self.metadata_tag_category_completion_active(),
        }
    }

    pub(super) fn reset_metadata_tag_completion_cycle(&mut self) {
        self.metadata.pending_tag_completion_query = None;
        self.metadata.tag_completion_cycle.reset();
    }
}
