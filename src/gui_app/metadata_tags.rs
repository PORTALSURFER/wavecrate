use super::GuiAppState;
use super::GuiMessage;
use radiant::prelude as ui;
use radiant::widgets::{DragHandleMessage, TextInputMessage};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    time::Instant,
};
use wavecrate::sample_sources::SampleSource;

#[cfg(test)]
pub(super) use types::MetadataTagCommit;
pub(super) use types::{
    MetadataTagCategoryGroup, MetadataTagCompletionOption, MetadataTagDisplayCategory,
    MetadataTagInputMode, MetadataTagPersistResult,
};
pub(super) use vocabulary::{
    commit_metadata_tag_text, inferred_metadata_tag_category_id_for_name,
    metadata_tag_category_order, normalize_metadata_tag,
};
#[cfg(test)]
pub(super) use vocabulary::{metadata_tag_category_id, metadata_tag_completion};

mod persistence;
mod types;
mod vocabulary;

use persistence::{
    load_persisted_metadata_tags_for_source, persist_metadata_tag_assignment,
    persist_metadata_tag_deletions,
};
#[cfg(test)]
pub(super) use persistence::{
    persist_metadata_tag_additions_for_tests, persist_metadata_tag_removals_for_tests,
};
use types::MetadataTagPersistRequest;
use vocabulary::{
    DEFAULT_METADATA_TAGS, METADATA_TAG_CATEGORIES, USER_EXTENSIBLE_METADATA_TAG_CATEGORIES,
    inferred_metadata_tag_category_id, metadata_tag_category_is_locked,
    metadata_tag_category_label_for_id, metadata_tag_completions_for_prefix,
    normalize_metadata_category_query, static_metadata_tag_category_id,
};

impl GuiAppState {
    pub(super) fn load_persisted_metadata_tags(
        sources: &[SampleSource],
    ) -> Result<HashMap<String, Vec<String>>, String> {
        let mut tags_by_file = HashMap::new();
        let mut errors = Vec::new();
        for source in sources {
            if let Err(error) =
                load_persisted_metadata_tags_for_source(&source.root, &mut tags_by_file)
            {
                errors.push(format!("{}: {error}", source.root.display()));
            }
        }
        if errors.is_empty() {
            Ok(tags_by_file)
        } else {
            Err(errors.join("; "))
        }
    }

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
        match message {
            TextInputMessage::Changed { value } => {
                self.metadata_tag_draft = value;
                self.reset_metadata_tag_completion_cycle();
            }
            TextInputMessage::Submitted { value } => {
                self.submit_metadata_tag_input(value, context);
            }
            TextInputMessage::CompletionRequested { value } => {
                self.metadata_tag_draft = value;
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

    pub(super) fn metadata_tag_completion_suffix(&self) -> Option<String> {
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

    pub(super) fn metadata_tag_completion_options(&self) -> Vec<MetadataTagCompletionOption> {
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

    pub(super) fn move_metadata_tag_completion_selection(&mut self, delta: i32) {
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
        let len = suggestions.len() as i32;
        self.metadata_tag_completion_prefix = Some(prefix);
        self.metadata_tag_completion_index = (current as i32 + delta).rem_euclid(len) as usize;
    }

    fn selected_metadata_tag_completion(&self) -> Option<String> {
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

    pub(super) fn metadata_tag_completion_active(&self) -> bool {
        if matches!(
            self.metadata_tag_input_mode,
            MetadataTagInputMode::Category { .. }
        ) {
            return !self.metadata_tag_category_suggestions().is_empty();
        }
        !self.metadata_tag_suggestions().is_empty()
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

    pub(super) fn categorized_metadata_tags(&self) -> Vec<MetadataTagCategoryGroup> {
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

    fn is_new_metadata_tag(&self, tag: &str) -> bool {
        !self.known_metadata_tags().iter().any(|known| known == tag)
    }

    fn metadata_tag_category_id(&self, tag: &str) -> &'static str {
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

    pub(super) fn metadata_tag_drag_active(&self) -> bool {
        self.metadata_tag_drag.is_some()
    }

    pub(super) fn metadata_tag_drop_hover(&self) -> Option<&str> {
        self.metadata_tag_drop_hover.as_deref()
    }

    pub(super) fn hover_metadata_tag_drop_category(&mut self, category_id: String) {
        if self.metadata_tag_drag.is_none() || metadata_tag_category_is_locked(category_id.as_str())
        {
            self.metadata_tag_drop_hover = None;
            return;
        }
        self.metadata_tag_drop_hover = Some(category_id);
    }

    pub(super) fn drag_metadata_tag(
        &mut self,
        tag: String,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if metadata_tag_category_is_locked(self.metadata_tag_category_id(&tag)) {
            self.metadata_tag_drag = None;
            self.metadata_tag_drop_hover = None;
            context.end_drag();
            self.sample_status = String::from("Playback Type tags are locked");
            return;
        }
        match drag {
            DragHandleMessage::Started { position } => {
                self.metadata_tag_drag = Some(tag.clone());
                self.metadata_tag_drop_hover = None;
                let width = (tag.chars().count() as f32 * 7.0 + 48.0).clamp(92.0, 180.0);
                context.begin_drag(ui::DragRequest::new(
                    ui::DragPreview::sized(
                        format!("Move {tag}"),
                        ui::Vector2::new(width, super::DRAG_PREVIEW_HEIGHT),
                    ),
                    position,
                ));
                self.sample_status = format!("Moving tag {tag}");
            }
            DragHandleMessage::Moved { .. } => {}
            DragHandleMessage::Ended { .. } => {
                self.metadata_tag_drag = None;
                self.metadata_tag_drop_hover = None;
                context.end_drag();
            }
        }
    }

    pub(super) fn drop_metadata_tag_on_category(
        &mut self,
        category_id: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let Some(tag) = self.metadata_tag_drag.clone() else {
            return;
        };
        self.metadata_tag_drag = None;
        self.metadata_tag_drop_hover = None;
        context.end_drag();
        if metadata_tag_category_is_locked(category_id.as_str()) {
            self.sample_status = String::from("Playback Type is locked");
            return;
        }
        if metadata_tag_category_is_locked(self.metadata_tag_category_id(&tag)) {
            self.sample_status = String::from("Playback Type tags are locked");
            return;
        }
        let Some(category_id) = static_metadata_tag_category_id(category_id.as_str()) else {
            return;
        };
        let previous_category = self.metadata_tag_category_id(&tag);
        if previous_category == category_id {
            self.sample_status = format!(
                "Tag {tag} is already in {}",
                metadata_tag_category_label_for_id(category_id).unwrap_or("this category")
            );
            return;
        }
        self.metadata_tag_dictionary
            .insert(tag.clone(), category_id.to_string());
        self.persist_user_configuration("metadata.tags.dictionary.move", Instant::now());
        self.sample_status = format!(
            "Moved tag {tag} to {}",
            metadata_tag_category_label_for_id(category_id).unwrap_or("category")
        );
    }

    pub(super) fn delete_metadata_tag_from_library(
        &mut self,
        tag: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if metadata_tag_category_is_locked(self.metadata_tag_category_id(&tag)) {
            self.sample_status = String::from("Playback Type tags are locked");
            return;
        }

        self.context_menu = None;
        self.metadata_tag_dictionary.remove(&tag);
        self.metadata_tag_drag = None;
        self.metadata_tag_drop_hover = None;
        if self.selected_metadata_tag.as_deref() == Some(tag.as_str()) {
            self.selected_metadata_tag = None;
        }

        let mut removed_count = 0usize;
        let mut requests = Vec::new();
        let affected_files = self
            .metadata_tags_by_file
            .iter()
            .filter_map(|(file_id, tags)| {
                tags.iter()
                    .any(|existing| existing == &tag)
                    .then_some(file_id.clone())
            })
            .collect::<Vec<_>>();
        for file_id in affected_files {
            let Some(file_tags) = self.metadata_tags_by_file.get_mut(&file_id) else {
                continue;
            };
            let before_len = file_tags.len();
            file_tags.retain(|existing| existing != &tag);
            let removed_here = before_len.saturating_sub(file_tags.len());
            if removed_here == 0 {
                continue;
            }
            removed_count += removed_here;
            if file_tags.is_empty() {
                self.metadata_tags_by_file.remove(&file_id);
            }
            let absolute_path = PathBuf::from(&file_id);
            if let Some((source_root, relative_path)) = self
                .folder_browser
                .source_relative_file_path(&absolute_path)
            {
                requests.push(MetadataTagPersistRequest {
                    absolute_path,
                    source_root,
                    relative_path,
                    tags: vec![tag.clone()],
                    assigned: false,
                });
            }
        }

        self.persist_user_configuration("metadata.tags.dictionary.delete", Instant::now());
        self.sample_status = if removed_count == 0 {
            format!("Deleted tag {tag}")
        } else {
            format!("Deleted tag {tag} from {removed_count} assignment(s)")
        };
        if !requests.is_empty() {
            context.spawn(
                "gui-metadata-tag-delete-persist",
                move || persist_metadata_tag_deletions(requests),
                GuiMessage::MetadataTagsPersisted,
            );
        }
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

    fn selected_metadata_tag_category(&self, value: &str) -> Option<&'static str> {
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
        let len = suggestions.len() as i32;
        self.metadata_tag_completion_prefix = Some(prefix);
        self.metadata_tag_completion_index = (current as i32 + delta).rem_euclid(len) as usize;
    }

    fn reset_metadata_tag_completion_cycle(&mut self) {
        self.metadata_tag_completion_prefix = None;
        self.metadata_tag_completion_index = 0;
    }

    pub(super) fn add_metadata_tags(
        &mut self,
        tags: Vec<String>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let Some(file_id) = self.folder_browser.selected_file_id().map(str::to_owned) else {
            self.sample_status = String::from("Select a sample before adding tags");
            return;
        };
        let absolute_path = PathBuf::from(&file_id);
        let Some((source_root, relative_path)) = self
            .folder_browser
            .source_relative_file_path(&absolute_path)
        else {
            self.sample_status = String::from("Selected sample is not inside a configured source");
            return;
        };
        let mut file_tags = self
            .metadata_tags_by_file
            .get(&file_id)
            .cloned()
            .unwrap_or_default();
        let mut added = Vec::new();
        for tag in tags {
            if file_tags.iter().any(|existing| existing == &tag) {
                continue;
            }
            file_tags.push(tag.clone());
            added.push(tag);
        }
        self.metadata_tags_by_file
            .insert(file_id.clone(), file_tags);
        match added.as_slice() {
            [] => {}
            [tag] => self.sample_status = format!("Added tag {tag}"),
            tags => self.sample_status = format!("Added {} tags", tags.len()),
        }
        if !added.is_empty() {
            let request = MetadataTagPersistRequest {
                absolute_path,
                source_root,
                relative_path,
                tags: added,
                assigned: true,
            };
            context.spawn(
                "gui-metadata-tag-persist",
                move || persist_metadata_tag_assignment(request),
                GuiMessage::MetadataTagsPersisted,
            );
        }
    }

    pub(super) fn toggle_metadata_tag(
        &mut self,
        tag: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if self
            .selected_metadata_tags()
            .iter()
            .any(|existing| existing == &tag)
        {
            self.remove_metadata_tag(tag, context);
        } else {
            self.add_metadata_tags(vec![tag], context);
        }
    }

    pub(super) fn remove_selected_metadata_tag(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let Some(tag) = self.selected_metadata_tag.clone() else {
            return;
        };
        self.remove_metadata_tag(tag, context);
    }

    fn remove_metadata_tag(&mut self, tag: String, context: &mut ui::UpdateContext<GuiMessage>) {
        let Some(file_id) = self.folder_browser.selected_file_id().map(str::to_owned) else {
            self.sample_status = String::from("Select a sample before removing tags");
            return;
        };
        let absolute_path = PathBuf::from(&file_id);
        let Some((source_root, relative_path)) = self
            .folder_browser
            .source_relative_file_path(&absolute_path)
        else {
            self.sample_status = String::from("Selected sample is not inside a configured source");
            return;
        };
        let Some(file_tags) = self.metadata_tags_by_file.get_mut(&file_id) else {
            return;
        };
        let before_len = file_tags.len();
        file_tags.retain(|existing| existing != &tag);
        if file_tags.len() == before_len {
            return;
        }
        if file_tags.is_empty() {
            self.metadata_tags_by_file.remove(&file_id);
        }
        if self.selected_metadata_tag.as_deref() == Some(tag.as_str()) {
            self.selected_metadata_tag = None;
        }
        self.sample_status = format!("Removed tag {tag}");
        let request = MetadataTagPersistRequest {
            absolute_path,
            source_root,
            relative_path,
            tags: vec![tag],
            assigned: false,
        };
        context.spawn(
            "gui-metadata-tag-persist",
            move || persist_metadata_tag_assignment(request),
            GuiMessage::MetadataTagsPersisted,
        );
    }

    pub(super) fn finish_metadata_tag_persist(&mut self, result: MetadataTagPersistResult) {
        if let Err(error) = result.result {
            self.sample_status = match result.tags.as_slice() {
                [tag] if result.assigned => format!("Tag {tag} not saved: {error}"),
                [tag] => format!("Tag {tag} not removed: {error}"),
                tags if result.assigned => format!("{} tags not saved: {error}", tags.len()),
                tags => format!("{} tags not removed: {error}", tags.len()),
            };
        }
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
