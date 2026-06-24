use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, MetadataMessage};
use crate::native_app::metadata::{
    MetadataTagDisplayCategory, MetadataTagSelectionState, metadata_tag_category_is_pinned,
    metadata_tag_pill_selection_style, metadata_tag_pill_style,
};

use super::{TagEditorFieldParts, identity};
use crate::native_app::app_chrome::library_browser::library_sidebar::tag_entry_layout::{
    TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_ITEM_GAP, TagEntryRowItem,
    metadata_tag_category_id_for_display, tag_pill_width,
};

pub(super) struct TagEntryRowContext<'a> {
    display_categories: &'a [MetadataTagDisplayCategory],
    draft: &'a str,
    input_placeholder: &'a str,
    completion_suffix: Option<&'a str>,
    selected_tag: Option<&'a str>,
    mixed_tags: &'a [String],
}

impl<'a> TagEntryRowContext<'a> {
    pub(super) fn from_field(field: &'a TagEditorFieldParts<'a>) -> Self {
        Self {
            display_categories: field.display_categories,
            draft: field.draft,
            input_placeholder: field.input_placeholder,
            completion_suffix: field.completion_suffix,
            selected_tag: field.selected_tag,
            mixed_tags: field.mixed_tags,
        }
    }
}

pub(super) fn tag_entry_row(
    row: Vec<TagEntryRowItem>,
    context: &TagEntryRowContext<'_>,
    row_index: usize,
) -> ui::View<GuiMessage> {
    ui::row(
        row.into_iter()
            .map(|item| match item {
                TagEntryRowItem::Accepted(tag) => accepted_tag_token(
                    tag.as_str(),
                    metadata_tag_category_id_for_display(tag.as_str(), context.display_categories),
                    context.selected_tag == Some(tag.as_str()),
                    context.mixed_tags.iter().any(|mixed| mixed == &tag),
                ),
                TagEntryRowItem::PendingCategory(tag) => pending_category_tag_token(tag.as_str()),
                TagEntryRowItem::Input(width) => tag_text_input(
                    context.draft,
                    context.input_placeholder,
                    context.completion_suffix,
                    width,
                ),
                TagEntryRowItem::LibraryToggle(width) => metadata_tag_library_toggle(width),
            })
            .collect::<Vec<_>>(),
    )
    .key(identity::tag_row_key(row_index))
    .height(TAG_FIELD_CONTROL_HEIGHT)
    .fill_width()
    .spacing(TAG_FIELD_ITEM_GAP)
}

fn tag_text_input(
    tag_draft: &str,
    placeholder: &str,
    completion_suffix: Option<&str>,
    width: f32,
) -> ui::View<GuiMessage> {
    let mut input = ui::text_input(tag_draft.to_string())
        .placeholder(placeholder)
        .underline();

    if let Some(suffix) = completion_suffix.filter(|suffix| !suffix.is_empty()) {
        input = input.completion_suffix(suffix);
    }

    input
        .message_event(|message| GuiMessage::Metadata(MetadataMessage::MetadataTagInput(message)))
        .id(identity::metadata_tag_input_id())
        .size(width, TAG_FIELD_CONTROL_HEIGHT)
}

fn accepted_tag_token(
    tag: &str,
    category_id: &str,
    selected: bool,
    mixed: bool,
) -> ui::View<GuiMessage> {
    let active = !mixed && (selected || metadata_tag_category_is_pinned(category_id));
    let style = if active {
        metadata_tag_pill_style(category_id, true)
    } else if mixed {
        metadata_tag_pill_selection_style(category_id, MetadataTagSelectionState::Mixed)
    } else {
        metadata_tag_pill_style(category_id, false)
    };
    let tag_for_input = tag.to_string();
    ui::interactive_badge(tag.to_string())
        .style(style)
        .active(active)
        .actions(ui::row_actions().primary_secondary_key(
            tag_for_input,
            |tag| GuiMessage::Metadata(MetadataMessage::SelectMetadataTag(tag)),
            |tag, position| {
                GuiMessage::Metadata(MetadataMessage::OpenMetadataTagContextMenu { tag, position })
            },
        ))
        .key(identity::accepted_tag_key(tag))
        .size(tag_pill_width(tag), TAG_FIELD_CONTROL_HEIGHT)
}

fn pending_category_tag_token(tag: &str) -> ui::View<GuiMessage> {
    ui::badge(tag.to_string())
        .subtle()
        .passive()
        .key(identity::pending_category_tag_key(tag))
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .size(tag_pill_width(tag), TAG_FIELD_CONTROL_HEIGHT)
}

fn metadata_tag_library_toggle(width: f32) -> ui::View<GuiMessage> {
    let toggle = ui::disclosure_button(false)
        .message(GuiMessage::Metadata(
            MetadataMessage::ToggleMetadataTagLibrary,
        ))
        .key(identity::TAG_LIBRARY_TOGGLE_KEY)
        .size(width, TAG_FIELD_CONTROL_HEIGHT);
    #[cfg(test)]
    {
        toggle.id(identity::metadata_tag_library_toggle_id())
    }
    #[cfg(not(test))]
    {
        toggle
    }
}
