use crate::native_app::app_chrome::view_models::library_sidebar::TagEditorViewModel;
use crate::native_app::metadata::metadata_tag_category_is_pinned;

use super::super::tag_entry_layout::{
    TagEntryFieldInput, TagEntryRowItem, metadata_tag_category_id_for_display,
    tag_field_content_width,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TagEditorProjection {
    pub(super) field: Option<TagEditorFieldProjection>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TagEditorFieldProjection {
    pub(super) rows: Vec<TagEntryRowProjection>,
    pub(super) layout: TagEditorFieldLayoutProjection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TagEditorFieldLayoutProjection {
    pub(super) field_height: f32,
    pub(super) content_height: f32,
    pub(super) requires_scroll: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TagEntryRowProjection {
    pub(super) items: Vec<TagEntryItemProjection>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TagEntryItemProjection {
    Accepted(TagTokenProjection),
    PendingCategory(TagPendingCategoryProjection),
    Input(TagInputProjection),
    LibraryToggle(TagLibraryToggleProjection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TagTokenProjection {
    pub(super) label: String,
    pub(super) category_id: String,
    pub(super) mixed: bool,
    pub(super) active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TagPendingCategoryProjection {
    pub(super) label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TagInputProjection {
    pub(super) draft: String,
    pub(super) placeholder: String,
    pub(super) completion_suffix: Option<String>,
    pub(super) width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TagLibraryToggleProjection {
    pub(super) width: f32,
}

impl TagEditorProjection {
    pub(super) fn from_model(
        model: &TagEditorViewModel,
        sidebar_width: f32,
        library_toggle_width: f32,
    ) -> Self {
        if !model.has_selected_file {
            return Self { field: None };
        }

        let content_width = tag_field_content_width(sidebar_width);
        let field = super::super::tag_entry_layout::TagEntryFieldProjection::from_input(
            TagEntryFieldInput {
                draft: model.draft.as_str(),
                tokens: model.tokens.as_slice(),
                pending_category_tag: model.pending_category_tag.as_deref(),
                placeholder: model.input_placeholder.as_str(),
                completion_suffix: model.completion_suffix.as_deref(),
                tags: model.tags.as_slice(),
                display_categories: model.display_categories.as_slice(),
                content_width,
                library_toggle_width: Some(library_toggle_width),
            },
        );

        Self {
            field: Some(TagEditorFieldProjection {
                rows: field
                    .rows
                    .into_iter()
                    .map(|row| project_row(row, model))
                    .collect(),
                layout: TagEditorFieldLayoutProjection {
                    field_height: field.layout.field_height,
                    content_height: field.layout.content_height,
                    requires_scroll: field.layout.requires_scroll,
                },
            }),
        }
    }
}

fn project_row(row: Vec<TagEntryRowItem>, model: &TagEditorViewModel) -> TagEntryRowProjection {
    TagEntryRowProjection {
        items: row
            .into_iter()
            .map(|item| project_row_item(item, model))
            .collect(),
    }
}

fn project_row_item(item: TagEntryRowItem, model: &TagEditorViewModel) -> TagEntryItemProjection {
    match item {
        TagEntryRowItem::Accepted(label) => {
            TagEntryItemProjection::Accepted(project_tag_token(label, model))
        }
        TagEntryRowItem::PendingCategory(label) => {
            TagEntryItemProjection::PendingCategory(TagPendingCategoryProjection { label })
        }
        TagEntryRowItem::Input(width) => TagEntryItemProjection::Input(TagInputProjection {
            draft: model.draft.clone(),
            placeholder: model.input_placeholder.clone(),
            completion_suffix: model.completion_suffix.clone(),
            width,
        }),
        TagEntryRowItem::LibraryToggle(width) => {
            TagEntryItemProjection::LibraryToggle(TagLibraryToggleProjection { width })
        }
    }
}

fn project_tag_token(label: String, model: &TagEditorViewModel) -> TagTokenProjection {
    let category_id =
        metadata_tag_category_id_for_display(&label, model.display_categories.as_slice());
    let selected = model.selected_tag.as_deref() == Some(label.as_str());
    let mixed = model.mixed_tags.iter().any(|mixed| mixed == &label);
    let active = !mixed && (selected || metadata_tag_category_is_pinned(category_id));

    TagTokenProjection {
        label,
        category_id: category_id.to_string(),
        mixed,
        active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::metadata::MetadataTagDisplayCategory;

    #[test]
    fn tag_editor_projection_hides_field_without_selected_file() {
        let mut model = tag_editor_model();
        model.has_selected_file = false;

        let projection = TagEditorProjection::from_model(&model, 260.0, 22.0);

        assert_eq!(projection.field, None);
    }

    #[test]
    fn tag_editor_projection_carries_tag_token_state() {
        let projection = TagEditorProjection::from_model(&tag_editor_model(), 360.0, 22.0);
        let field = projection
            .field
            .expect("selected file should project field");
        let accepted = field
            .rows
            .iter()
            .flat_map(|row| row.items.iter())
            .filter_map(|item| match item {
                TagEntryItemProjection::Accepted(tag) => Some(tag),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(accepted.iter().any(|tag| {
            tag.label == "kick" && tag.category_id == "sound-type" && tag.active && !tag.mixed
        }));
        assert!(accepted.iter().any(|tag| {
            tag.label == "loop" && tag.category_id == "playback-type" && tag.active && !tag.mixed
        }));
        assert!(accepted.iter().any(|tag| {
            tag.label == "warm" && tag.category_id == "character" && !tag.active && tag.mixed
        }));
    }

    #[test]
    fn tag_editor_projection_carries_input_and_library_toggle_intent() {
        let projection = TagEditorProjection::from_model(&tag_editor_model(), 360.0, 22.0);
        let field = projection
            .field
            .expect("selected file should project field");
        let input = field
            .rows
            .iter()
            .flat_map(|row| row.items.iter())
            .find_map(|item| match item {
                TagEntryItemProjection::Input(input) => Some(input),
                _ => None,
            })
            .expect("tag field should include text input");

        assert_eq!(input.draft, "sou");
        assert_eq!(input.placeholder, "Add tag");
        assert_eq!(input.completion_suffix.as_deref(), Some("nd"));
        assert!(input.width > 0.0);
        assert!(
            field
                .rows
                .iter()
                .flat_map(|row| row.items.iter())
                .any(|item| matches!(
                    item,
                    TagEntryItemProjection::LibraryToggle(TagLibraryToggleProjection { width })
                    if *width == 22.0
                ))
        );
        assert!(field.layout.field_height >= field.layout.content_height);
    }

    #[test]
    fn tag_editor_projection_keeps_pending_category_with_input() {
        let mut model = tag_editor_model();
        model.pending_category_tag = Some("drum".to_string());

        let projection = TagEditorProjection::from_model(&model, 360.0, 22.0);
        let field = projection
            .field
            .expect("selected file should project field");

        assert!(field.rows.iter().any(|row| {
            let has_pending = row.items.iter().any(|item| {
                matches!(
                    item,
                    TagEntryItemProjection::PendingCategory(TagPendingCategoryProjection { label })
                    if label == "drum ->"
                )
            });
            let has_input = row
                .items
                .iter()
                .any(|item| matches!(item, TagEntryItemProjection::Input(_)));
            has_pending && has_input
        }));
    }

    fn tag_editor_model() -> TagEditorViewModel {
        TagEditorViewModel {
            has_selected_file: true,
            draft: "sou".to_string(),
            tokens: vec!["loop".to_string()],
            pending_category_tag: None,
            input_placeholder: "Add tag".to_string(),
            completion_suffix: Some("nd".to_string()),
            tags: vec!["kick".to_string(), "warm".to_string()],
            mixed_tags: vec!["warm".to_string()],
            display_categories: vec![
                MetadataTagDisplayCategory {
                    tag: "loop".to_string(),
                    category_id: "playback-type",
                },
                MetadataTagDisplayCategory {
                    tag: "kick".to_string(),
                    category_id: "sound-type",
                },
                MetadataTagDisplayCategory {
                    tag: "warm".to_string(),
                    category_id: "character",
                },
            ],
            selected_tag: Some("kick".to_string()),
        }
    }
}
