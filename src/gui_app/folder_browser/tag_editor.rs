use radiant::prelude as ui;

use crate::gui_app::metadata_tags::{MetadataTagCompletionOption, MetadataTagDisplayCategory};
use crate::gui_app::metadata_tags::{metadata_tag_category_is_pinned, metadata_tag_category_style};

use super::GuiMessage;
use super::tag_completion::tag_completion_panel_layer;
use super::tag_entry_layout::{
    TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_ITEM_GAP, TAG_FIELD_LINE_GAP, TagEntryRowItem,
    capped_rows_height, metadata_tag_category_id_for_display, order_metadata_tags_for_display,
    rows_height, tag_field_requires_scroll, tag_field_rows, tag_input_display_value,
    tag_input_width, tag_input_width_for_placeholder, tag_pill_width,
};

const METADATA_TAG_INPUT_ID: u64 = 0x5743_0000_0000_5447;
pub(super) fn metadata_section(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_pending_category_tag: Option<&str>,
    tag_input_placeholder: &str,
    tag_completion_suffix: Option<&str>,
    tag_completion_options: &[MetadataTagCompletionOption],
    tags: &[String],
    tag_display_categories: &[MetadataTagDisplayCategory],
    selected_metadata_tag: Option<&str>,
    tag_field_content_width: f32,
    tag_field_height: f32,
    has_selected_file: bool,
) -> ui::View<GuiMessage> {
    if !has_selected_file {
        return metadata_sidebar_panel(ui::empty().fill_width(), 12.0);
    }

    let content_height = 25.0 + tag_field_height;
    let section_height = 38.0 + tag_field_height;
    let mut layers = vec![
        ui::column([
            ui::row([
                ui::text(format!("Tags ({})", tags.len()))
                    .height(22.0)
                    .fill_width(),
                ui::disclosure_button(false)
                    .message(GuiMessage::ToggleMetadataTagLibrary)
                    .key("metadata-tag-library-toggle")
                    .size(24.0, 20.0),
            ])
            .spacing(4.0)
            .fill_width()
            .height(22.0)
            .key("metadata-tag-library-toggle-row"),
            tag_entry_field(
                tag_draft,
                tag_tokens,
                tag_pending_category_tag,
                tag_input_placeholder,
                tag_completion_suffix,
                tags,
                tag_display_categories,
                selected_metadata_tag,
                tag_field_height,
                tag_field_content_width,
            )
            .key("metadata-tag-entry-field")
            .fill_width()
            .height(tag_field_height),
        ])
        .fill_width()
        .spacing(3.0),
    ];
    if !tag_completion_options.is_empty() {
        layers.push(tag_completion_panel_layer(
            tag_completion_options,
            tag_field_content_width,
            content_height,
            tag_field_height,
        ));
    }
    metadata_sidebar_panel(
        ui::stack_layers(layers).fill_width().height(content_height),
        section_height,
    )
}

fn tag_entry_field(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_pending_category_tag: Option<&str>,
    tag_input_placeholder: &str,
    tag_completion_suffix: Option<&str>,
    tags: &[String],
    tag_display_categories: &[MetadataTagDisplayCategory],
    selected_metadata_tag: Option<&str>,
    height: f32,
    content_width: f32,
) -> ui::View<GuiMessage> {
    let mut visible_tags = tags.to_vec();
    for token in tag_tokens {
        if !visible_tags.iter().any(|tag| tag == token) {
            visible_tags.push(token.clone());
        }
    }
    order_metadata_tags_for_display(&mut visible_tags, tag_display_categories);

    let pending_category_tag = tag_pending_category_tag.map(str::to_string);
    let display_value = tag_input_display_value(tag_draft, tag_completion_suffix);
    let input_width = if pending_category_tag.is_some() {
        tag_input_width(display_value.as_str())
    } else {
        tag_input_width_for_placeholder(display_value.as_str(), tag_input_placeholder)
    };
    let rows = tag_field_rows(
        &visible_tags,
        tag_display_categories,
        pending_category_tag.as_deref(),
        input_width,
        content_width,
    );
    let row_count = rows.len();
    let content = ui::column(
        rows.into_iter()
            .enumerate()
            .map(|(row_index, row)| {
                tag_entry_row(
                    row,
                    tag_display_categories,
                    tag_draft,
                    tag_input_placeholder,
                    tag_completion_suffix,
                    selected_metadata_tag,
                    row_index,
                )
            })
            .collect::<Vec<_>>(),
    )
    .fill_width()
    .height(rows_height(row_count))
    .spacing(TAG_FIELD_LINE_GAP);

    if tag_field_requires_scroll(row_count) {
        ui::scroll(content)
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
            .padding(3.0)
            .fill_width()
            .height(height)
    } else {
        content.fill_width().height(height)
    }
}

pub(super) fn tag_field_height(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_pending_category_tag: Option<&str>,
    tag_completion_suffix: Option<&str>,
    tags: &[String],
    tag_display_categories: &[MetadataTagDisplayCategory],
    content_width: f32,
) -> f32 {
    let mut visible_tags = tags.to_vec();
    for token in tag_tokens {
        if !visible_tags.iter().any(|tag| tag == token) {
            visible_tags.push(token.clone());
        }
    }
    order_metadata_tags_for_display(&mut visible_tags, tag_display_categories);
    let input_width =
        tag_input_width(tag_input_display_value(tag_draft, tag_completion_suffix).as_str());
    let rows = tag_field_rows(
        &visible_tags,
        tag_display_categories,
        tag_pending_category_tag,
        input_width,
        content_width,
    )
    .len();
    capped_rows_height(rows)
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
        .message_event(GuiMessage::MetadataTagInput)
        .id(METADATA_TAG_INPUT_ID)
        .key("metadata-tag-input")
        .size(width, TAG_FIELD_CONTROL_HEIGHT)
}

fn tag_entry_row(
    row: Vec<TagEntryRowItem>,
    tag_display_categories: &[MetadataTagDisplayCategory],
    tag_draft: &str,
    tag_input_placeholder: &str,
    tag_completion_suffix: Option<&str>,
    selected_metadata_tag: Option<&str>,
    row_index: usize,
) -> ui::View<GuiMessage> {
    ui::row(
        row.into_iter()
            .map(|item| match item {
                TagEntryRowItem::Accepted(tag) => accepted_tag_token(
                    tag.as_str(),
                    metadata_tag_category_id_for_display(tag.as_str(), tag_display_categories),
                    selected_metadata_tag == Some(tag.as_str()),
                ),
                TagEntryRowItem::PendingCategory(tag) => pending_category_tag_token(tag.as_str()),
                TagEntryRowItem::Input(width) => tag_text_input(
                    tag_draft,
                    tag_input_placeholder,
                    tag_completion_suffix,
                    width,
                ),
            })
            .collect::<Vec<_>>(),
    )
    .key(format!("metadata-tag-row-{row_index}"))
    .height(TAG_FIELD_CONTROL_HEIGHT)
    .fill_width()
    .spacing(TAG_FIELD_ITEM_GAP)
}

fn accepted_tag_token(tag: &str, category_id: &str, selected: bool) -> ui::View<GuiMessage> {
    let style = metadata_tag_category_style(category_id, selected);
    let tag_for_input = tag.to_string();
    let mut badge = ui::interactive_badge(tag.to_string())
        .style(style)
        .actions(
            ui::InteractiveRowActions::new()
                .secondary({
                    let tag = tag_for_input.clone();
                    move |position| GuiMessage::OpenMetadataTagContextMenu {
                        tag: tag.clone(),
                        position,
                    }
                })
                .activate(move || GuiMessage::SelectMetadataTag(tag_for_input.clone())),
        )
        .key(format!("metadata-tag-accepted-{tag}"))
        .size(tag_pill_width(tag), TAG_FIELD_CONTROL_HEIGHT);
    if !selected && !metadata_tag_category_is_pinned(category_id) {
        badge = badge.subtle();
    }
    badge
}

fn pending_category_tag_token(tag: &str) -> ui::View<GuiMessage> {
    ui::badge(tag.to_string())
        .subtle()
        .passive()
        .key(format!("metadata-tag-pending-category-{tag}"))
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .size(tag_pill_width(tag), TAG_FIELD_CONTROL_HEIGHT)
}

fn metadata_sidebar_panel(content: ui::View<GuiMessage>, height: f32) -> ui::View<GuiMessage> {
    content
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
        .padding(6.0)
        .fill_width()
        .height(height)
}
