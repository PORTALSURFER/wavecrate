use super::GuiAppState;
use super::GuiMessage;
use radiant::prelude as ui;
use radiant::widgets::{TextInputMessage, TextInputMessageKind};
use std::time::Instant;

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
mod persistence;
mod types;
mod vocabulary;

use persistence::load_persisted_metadata_tags_for_source;
#[cfg(test)]
pub(super) use persistence::{
    persist_metadata_tag_additions_for_tests, persist_metadata_tag_removals_for_tests,
};
impl GuiAppState {
    pub(super) fn refresh_persisted_metadata_tags_for_source(&mut self, source_id: &str) {
        let Some(root) = self.folder_browser.source_root_path(source_id) else {
            return;
        };
        if let Err(error) =
            load_persisted_metadata_tags_for_source(&root, &mut self.metadata_tags_by_file)
        {
            self.sample_status = format!("Tags not loaded: {error}");
        }
    }

    pub(super) fn selected_metadata_tags(&self) -> &[String] {
        self.folder_browser
            .selected_file_id()
            .and_then(|file_id| self.metadata_tags_by_file.get(file_id))
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
            self.selected_metadata_tag = Some(tag);
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
            TextInputMessageKind::Changed | TextInputMessageKind::CompletionRequested => {
                self.metadata_tag_draft = parts.value.to_owned();
                self.reset_metadata_tag_completion_cycle();
            }
        }
    }

    fn submit_metadata_tag_input(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if matches!(
            self.metadata_tag_input_mode,
            MetadataTagInputMode::Category { .. }
        ) {
            self.submit_metadata_tag_category(value, context);
        } else {
            self.submit_metadata_tag_value(value, context);
        }
    }

    fn submit_metadata_tag_value(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let mut commit = commit_metadata_tag_text(&value);
        let mut tags = std::mem::take(&mut self.metadata_tag_tokens);
        if tags.is_empty()
            && commit.tags.len() <= 1
            && let Some(tag) = self.selected_metadata_tag_completion()
        {
            tags.push(tag);
            commit.tags.clear();
        }
        tags.append(&mut commit.tags);
        if tags.len() == 1 && self.is_new_metadata_tag(tags[0].as_str()) {
            let tag = tags.remove(0);
            self.metadata_tag_input_mode = MetadataTagInputMode::Category {
                pending_tag: tag.clone(),
            };
            self.metadata_tag_draft.clear();
            self.reset_metadata_tag_completion_cycle();
            self.sample_status = format!("Choose a category for {tag}");
            return;
        }
        self.metadata_tag_draft.clear();
        self.reset_metadata_tag_completion_cycle();
        self.add_metadata_tags(tags, context);
    }

    fn submit_metadata_tag_category(
        &mut self,
        value: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let MetadataTagInputMode::Category { pending_tag } = self.metadata_tag_input_mode.clone()
        else {
            return;
        };
        let Some(category_id) = self.selected_metadata_tag_category(value.as_str()) else {
            self.sample_status = format!("Choose a category for {pending_tag}");
            return;
        };
        self.metadata_tag_dictionary
            .insert(pending_tag.clone(), category_id.to_string());
        self.metadata_tag_input_mode = MetadataTagInputMode::Tag;
        self.metadata_tag_draft.clear();
        self.reset_metadata_tag_completion_cycle();
        self.persist_user_configuration("metadata.tags.dictionary.persist", Instant::now());
        self.add_metadata_tags(vec![pending_tag], context);
    }

    pub(super) fn metadata_tag_input_placeholder(&self) -> &'static str {
        match self.metadata_tag_input_mode {
            MetadataTagInputMode::Tag => "add tag",
            MetadataTagInputMode::Category { .. } => "select group/parent tag",
        }
    }

    pub(super) fn pending_metadata_tag_category_tag(&self) -> Option<&str> {
        match &self.metadata_tag_input_mode {
            MetadataTagInputMode::Tag => None,
            MetadataTagInputMode::Category { pending_tag } => Some(pending_tag.as_str()),
        }
    }

    pub(super) fn cancel_metadata_tag_entry(&mut self) {
        self.metadata_tag_draft.clear();
        self.metadata_tag_tokens.clear();
        self.metadata_tag_input_mode = MetadataTagInputMode::Tag;
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
