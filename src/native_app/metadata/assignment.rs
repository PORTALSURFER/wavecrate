use super::persistence::persist_metadata_tag_assignment;
use super::types::{MetadataTagPersistRequest, MetadataTagPersistResult};
use super::{GuiMessage, NativeAppState};
use radiant::prelude as ui;
use std::path::PathBuf;

impl NativeAppState {
    pub(in crate::native_app) fn add_metadata_tags(
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
            .metadata
            .tags_by_file
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
        self.metadata
            .tags_by_file
            .insert(file_id.clone(), file_tags);
        self.retain_visible_file_selection_after_metadata_tag_change();
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

    pub(in crate::native_app) fn toggle_metadata_tag(
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

    pub(in crate::native_app) fn remove_selected_metadata_tag(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let Some(tag) = self.metadata.selected_tag.clone() else {
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
        let Some(file_tags) = self.metadata.tags_by_file.get_mut(&file_id) else {
            return;
        };
        let before_len = file_tags.len();
        file_tags.retain(|existing| existing != &tag);
        if file_tags.len() == before_len {
            return;
        }
        if file_tags.is_empty() {
            self.metadata.tags_by_file.remove(&file_id);
        }
        if self.metadata.selected_tag.as_deref() == Some(tag.as_str()) {
            self.metadata.selected_tag = None;
        }
        self.retain_visible_file_selection_after_metadata_tag_change();
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

    pub(in crate::native_app) fn finish_metadata_tag_persist(
        &mut self,
        result: MetadataTagPersistResult,
    ) {
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
