use super::persistence::persist_metadata_tag_deletions;
use super::types::MetadataTagPersistRequest;
use super::vocabulary::{
    metadata_tag_category_is_locked, metadata_tag_category_label_for_id,
    static_metadata_tag_category_id,
};
use super::{GuiAppState, GuiMessage};
use crate::gui_app::DRAG_PREVIEW_HEIGHT;
use radiant::prelude as ui;
use radiant::widgets::DragHandleMessage;
use std::{path::PathBuf, time::Instant};
impl GuiAppState {
    pub(in crate::gui_app) fn metadata_tag_drag_active(&self) -> bool {
        self.metadata_tag_drag.is_some()
    }

    pub(in crate::gui_app) fn metadata_tag_drop_hover(&self) -> Option<&str> {
        self.metadata_tag_drop_hover.as_deref()
    }

    /// Returns the tag currently being dragged from the metadata-tag library.
    pub(in crate::gui_app) fn dragged_metadata_tag(&self) -> Option<&str> {
        self.metadata_tag_drag.as_deref()
    }

    pub(in crate::gui_app) fn hover_metadata_tag_drop_category(&mut self, category_id: String) {
        if self.metadata_tag_drag.is_none() || metadata_tag_category_is_locked(category_id.as_str())
        {
            self.metadata_tag_drop_hover = None;
            return;
        }
        self.metadata_tag_drop_hover = Some(category_id);
    }

    pub(in crate::gui_app) fn drag_metadata_tag(
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
        if let Some(position) = drag.started_position() {
            self.metadata_tag_drag = Some(tag.clone());
            self.metadata_tag_drop_hover = None;
            context.begin_drag(ui::DragRequest::new(
                ui::DragPreview::text_sized(
                    format!("Move {tag}"),
                    ui::DragPreviewTextSizing::new(DRAG_PREVIEW_HEIGHT)
                        .horizontal_padding(48.0)
                        .min_width(92.0)
                        .max_width(180.0),
                ),
                position,
            ));
            self.sample_status = format!("Moving tag {tag}");
        } else if drag.is_finished() {
            self.metadata_tag_drag = None;
            self.metadata_tag_drop_hover = None;
            context.end_drag();
        }
    }

    pub(in crate::gui_app) fn drop_metadata_tag_on_category(
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

    pub(in crate::gui_app) fn delete_metadata_tag_from_library(
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

        self.retain_visible_file_selection_after_metadata_tag_change();
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
}
