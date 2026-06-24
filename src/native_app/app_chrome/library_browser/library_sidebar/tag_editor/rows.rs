use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, MetadataMessage};
use crate::native_app::metadata::{
    MetadataTagSelectionState, metadata_tag_pill_selection_style, metadata_tag_pill_style,
};

use super::identity;
use super::projection::{
    TagEntryItemProjection, TagEntryRowProjection, TagInputProjection, TagTokenProjection,
};
use crate::native_app::app_chrome::library_browser::library_sidebar::tag_entry_layout::{
    TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_ITEM_GAP, tag_pill_width,
};

pub(super) fn tag_entry_row(row: TagEntryRowProjection, row_index: usize) -> ui::View<GuiMessage> {
    ui::row(
        row.items
            .into_iter()
            .map(|item| match item {
                TagEntryItemProjection::Accepted(tag) => accepted_tag_token(tag),
                TagEntryItemProjection::PendingCategory(tag) => {
                    pending_category_tag_token(tag.label.as_str())
                }
                TagEntryItemProjection::Input(input) => tag_text_input(input),
                TagEntryItemProjection::LibraryToggle(toggle) => {
                    metadata_tag_library_toggle(toggle.width)
                }
            })
            .collect::<Vec<_>>(),
    )
    .key(identity::tag_row_key(row_index))
    .height(TAG_FIELD_CONTROL_HEIGHT)
    .fill_width()
    .spacing(TAG_FIELD_ITEM_GAP)
}

fn tag_text_input(projection: TagInputProjection) -> ui::View<GuiMessage> {
    let mut input = ui::text_input(projection.draft)
        .placeholder(projection.placeholder)
        .underline();

    if let Some(suffix) = projection
        .completion_suffix
        .filter(|suffix| !suffix.is_empty())
    {
        input = input.completion_suffix(suffix);
    }

    input
        .message_event(|message| GuiMessage::Metadata(MetadataMessage::MetadataTagInput(message)))
        .id(identity::metadata_tag_input_id())
        .size(projection.width, TAG_FIELD_CONTROL_HEIGHT)
}

fn accepted_tag_token(tag: TagTokenProjection) -> ui::View<GuiMessage> {
    let style = if tag.active {
        metadata_tag_pill_style(tag.category_id.as_str(), true)
    } else if tag.mixed {
        metadata_tag_pill_selection_style(
            tag.category_id.as_str(),
            MetadataTagSelectionState::Mixed,
        )
    } else {
        metadata_tag_pill_style(tag.category_id.as_str(), false)
    };
    let tag_for_input = tag.label.clone();
    ui::interactive_badge(tag.label.clone())
        .style(style)
        .active(tag.active)
        .actions(ui::row_actions().primary_secondary_key(
            tag_for_input,
            |tag| GuiMessage::Metadata(MetadataMessage::SelectMetadataTag(tag)),
            |tag, position| {
                GuiMessage::Metadata(MetadataMessage::OpenMetadataTagContextMenu { tag, position })
            },
        ))
        .key(identity::accepted_tag_key(&tag.label))
        .size(tag_pill_width(&tag.label), TAG_FIELD_CONTROL_HEIGHT)
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
