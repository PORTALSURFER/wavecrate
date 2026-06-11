use crate::native_app::app::GuiMessage;
use crate::native_app::app::MetadataMessage;
use crate::native_app::app::NativeAppState;
use radiant::prelude as ui;
use radiant::widgets::{TextInputMessage, TextInputMessageKind};
use std::time::Instant;

pub(super) use metrics::{metadata_tag_input_width_policy, metadata_tag_pill_width};
pub(in crate::native_app) use style::{metadata_tag_category_is_pinned, metadata_tag_pill_style};
#[cfg(test)]
pub(super) use types::MetadataTagCommit;
pub(super) use types::{
    MetadataTagCategoryGroup, MetadataTagCompletionOption, MetadataTagDisplayCategory,
    MetadataTagInputMode, MetadataTagPersistResult,
};
pub(super) use vocabulary::{
    commit_metadata_tag_text, inferred_metadata_tag_category_id_for_name,
    metadata_tag_category_order,
};
#[cfg(test)]
pub(super) use vocabulary::{
    metadata_tag_category_id, metadata_tag_completion, normalize_metadata_tag,
};

mod assignment;
mod completion;
mod library;
mod metrics;
mod persistence;
mod style;
mod types;
mod vocabulary;

use persistence::load_persisted_metadata_tags_for_source;
#[cfg(test)]
pub(super) use persistence::{
    persist_metadata_tag_additions_for_tests, persist_metadata_tag_removals_for_tests,
};
impl NativeAppState {
    pub(super) fn refresh_persisted_metadata_tags_for_source(&mut self, source_id: &str) {
        let Some(root) = self.library.folder_browser.source_root_path(source_id) else {
            return;
        };
        if let Err(error) =
            load_persisted_metadata_tags_for_source(&root, &mut self.metadata.tags_by_file)
        {
            self.ui.status.sample = format!("Tags not loaded: {error}");
        }
    }

    pub(super) fn retain_visible_file_selection_after_metadata_tag_change(&mut self) {
        self.library
            .folder_browser
            .retain_visible_file_selection_after_tag_filter(&self.metadata.tags_by_file);
    }

    pub(super) fn selected_metadata_tags(&self) -> &[String] {
        self.library
            .folder_browser
            .selected_file_id()
            .and_then(|file_id| self.metadata.tags_by_file.get(file_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(super) fn selected_metadata_tag_display_categories(
        &self,
    ) -> Vec<MetadataTagDisplayCategory> {
        self.selected_metadata_tags()
            .iter()
            .map(|tag| MetadataTagDisplayCategory {
                tag: tag.clone(),
                category_id: self.metadata_tag_category_id(tag),
            })
            .collect()
    }

    pub(super) fn select_metadata_tag(&mut self, tag: String) {
        if self
            .selected_metadata_tags()
            .iter()
            .any(|existing| existing == &tag)
        {
            self.metadata.selected_tag = Some(tag);
        }
    }

    pub(super) fn apply_metadata_tag_input(
        &mut self,
        message: TextInputMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let parts = message.parts();
        match parts.kind {
            TextInputMessageKind::Submitted => {
                self.submit_metadata_tag_input(parts.value.to_owned(), context);
            }
            TextInputMessageKind::Changed => {
                self.metadata.tag_draft = parts.value.to_owned();
                self.reset_metadata_tag_completion_cycle();
            }
            TextInputMessageKind::CompletionRequested => {
                self.metadata.tag_draft = parts.value.to_owned();
                self.activate_metadata_tag_completion();
            }
        }
    }

    pub(super) fn focus_metadata_tag_input(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        context.focus(crate::native_app::ui::ids::METADATA_TAG_INPUT_ID);
    }

    fn submit_metadata_tag_input(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if matches!(
            self.metadata.tag_input_mode,
            MetadataTagInputMode::Category { .. }
        ) {
            self.submit_metadata_tag_category(value, context);
        } else {
            self.submit_metadata_tag_value(value, context);
        }
    }

    pub(super) fn submit_selected_metadata_tag_completion(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if matches!(
            self.metadata.tag_input_mode,
            MetadataTagInputMode::Category { .. }
        ) {
            self.submit_metadata_tag_category_value(value, context);
        } else {
            self.submit_metadata_tag_input(value, context);
        }
    }

    fn submit_metadata_tag_value(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let mut commit = commit_metadata_tag_text(&value);
        let mut tags = std::mem::take(&mut self.metadata.tag_tokens);
        if tags.is_empty()
            && commit.tags.len() <= 1
            && let Some(tag) = self.explicit_metadata_tag_completion()
        {
            tags.push(tag);
            commit.tags.clear();
        }
        tags.append(&mut commit.tags);
        if tags.len() == 1 && self.is_new_metadata_tag(tags[0].as_str()) {
            let tag = tags.remove(0);
            self.metadata.tag_input_mode = MetadataTagInputMode::Category {
                pending_tag: tag.clone(),
            };
            self.metadata.tag_draft.clear();
            self.reset_metadata_tag_completion_cycle();
            self.ui.status.sample = format!("Choose a category for {tag}");
            return;
        }
        self.metadata.tag_draft.clear();
        self.reset_metadata_tag_completion_cycle();
        self.add_metadata_tags(tags, context);
    }

    fn submit_metadata_tag_category(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let MetadataTagInputMode::Category { pending_tag } = self.metadata.tag_input_mode.clone()
        else {
            return;
        };
        let Some(category_id) = self.selected_metadata_tag_category(value.as_str()) else {
            self.ui.status.sample = format!("Choose a category for {pending_tag}");
            return;
        };
        self.commit_metadata_tag_category(pending_tag, category_id, context);
    }

    fn submit_metadata_tag_category_value(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let MetadataTagInputMode::Category { pending_tag } = self.metadata.tag_input_mode.clone()
        else {
            return;
        };
        let Some(category_id) = self.metadata_tag_category_for_value(value.as_str()) else {
            self.ui.status.sample = format!("Choose a category for {pending_tag}");
            return;
        };
        self.commit_metadata_tag_category(pending_tag, category_id, context);
    }

    fn commit_metadata_tag_category(
        &mut self,
        pending_tag: String,
        category_id: &str,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.metadata
            .tag_dictionary
            .insert(pending_tag.clone(), category_id.to_string());
        self.metadata.tag_input_mode = MetadataTagInputMode::Tag;
        self.metadata.tag_draft.clear();
        self.reset_metadata_tag_completion_cycle();
        self.persist_user_configuration("metadata.tags.dictionary.persist", Instant::now());
        self.add_metadata_tags(vec![pending_tag], context);
    }

    pub(super) fn metadata_tag_input_placeholder(&self) -> &'static str {
        match self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => "add tag",
            MetadataTagInputMode::Category { .. } => "select group/parent tag",
        }
    }

    pub(super) fn pending_metadata_tag_category_tag(&self) -> Option<&str> {
        match &self.metadata.tag_input_mode {
            MetadataTagInputMode::Tag => None,
            MetadataTagInputMode::Category { pending_tag } => Some(pending_tag.as_str()),
        }
    }

    pub(super) fn cancel_metadata_tag_entry(&mut self) {
        self.metadata.tag_draft.clear();
        self.metadata.tag_tokens.clear();
        self.metadata.tag_input_mode = MetadataTagInputMode::Tag;
        self.reset_metadata_tag_completion_cycle();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_tags_normalize_to_single_token_values() {
        assert_eq!(
            normalize_metadata_tag("Deep Kick 01"),
            Some(String::from("deep-kick-01"))
        );
        assert_eq!(
            normalize_metadata_tag("  metal_floor  "),
            Some(String::from("metal_floor"))
        );
        assert_eq!(normalize_metadata_tag("!!!"), None);
    }

    #[test]
    fn submitted_tag_input_commits_delimited_tags() {
        assert_eq!(
            commit_metadata_tag_text("kick, warm tone"),
            MetadataTagCommit {
                tags: vec![String::from("kick"), String::from("warm-tone")],
                remainder: String::new(),
            }
        );
    }

    #[test]
    fn metadata_tag_completion_matches_known_tag_prefix() {
        assert_eq!(
            metadata_tag_completion("ki", ["warm", "kick", "kicker"].into_iter()),
            Some(String::from("kick"))
        );
        assert_eq!(metadata_tag_completion("zz", ["kick"].into_iter()), None);
    }

    #[test]
    fn metadata_tag_category_matches_target_category_vocabulary() {
        assert_eq!(metadata_tag_category_id("one-shot"), "playback-type");
        assert_eq!(metadata_tag_category_id("hat"), "sound-type");
        assert_eq!(metadata_tag_category_id("bright"), "character");
        assert_eq!(metadata_tag_category_id("prefix-artist"), "prefix");
        assert_eq!(metadata_tag_category_id("dorian"), "tuning-scale");
        assert_eq!(metadata_tag_category_id("custom-texture"), "character");
    }
}
