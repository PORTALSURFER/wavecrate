use radiant::{
    prelude as ui,
    widgets::{WidgetStyle, WidgetTone},
};

use crate::gui_app::metadata_tags::{MetadataTagCompletionOption, MetadataTagDisplayCategory};

use super::GuiMessage;
use super::tag_completion::tag_completion_panel_layer;
use super::tag_entry_layout::{
    MAX_TAG_FIELD_ROWS, TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_HORIZONTAL_CHROME, TAG_FIELD_ITEM_GAP,
    TAG_FIELD_LINE_GAP, TAG_FIELD_VERTICAL_CHROME, TagEntryRowItem,
    metadata_tag_category_id_for_display, order_metadata_tags_for_display, rows_height,
    tag_field_rows, tag_input_display_value, tag_input_width, tag_input_width_for_placeholder,
    tag_pill_width,
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
        return metadata_sidebar_panel(ui::spacer().height(0.0).fill_width(), 12.0);
    }

    let content_height = 25.0 + tag_field_height;
    let section_height = 38.0 + tag_field_height;
    let mut layers = vec![
        ui::column([
            ui::row([
                ui::text(format!("Tags ({})", tags.len()))
                    .height(22.0)
                    .fill_width(),
                ui::button(">")
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
        ui::stack(layers).fill_width().height(content_height),
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

    if row_count > MAX_TAG_FIELD_ROWS {
        ui::scroll(content)
            .style(WidgetStyle {
                tone: WidgetTone::Neutral,
                prominence: ui::WidgetProminence::Subtle,
            })
            .padding(3.0)
            .fill_width()
            .height(height)
    } else {
        content.fill_width().height(height)
    }
}

pub(super) fn tag_field_content_width(sidebar_width: f32) -> f32 {
    (sidebar_width - TAG_FIELD_HORIZONTAL_CHROME).max(120.0)
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
    rows_height(rows.clamp(1, MAX_TAG_FIELD_ROWS)) + TAG_FIELD_VERTICAL_CHROME
}

fn tag_text_input(
    tag_draft: &str,
    placeholder: &str,
    completion_suffix: Option<&str>,
    width: f32,
) -> ui::View<GuiMessage> {
    let input = ui::text_input(tag_draft.to_string())
        .placeholder(placeholder)
        .underline()
        .message_event(GuiMessage::MetadataTagInput)
        .id(METADATA_TAG_INPUT_ID)
        .key("metadata-tag-input")
        .sizing(ui::WidgetSizing::fixed(ui::Vector2::new(
            width,
            TAG_FIELD_CONTROL_HEIGHT,
        )))
        .height(TAG_FIELD_CONTROL_HEIGHT)
        .width(width);

    let Some(suffix) = completion_suffix.filter(|suffix| !suffix.is_empty()) else {
        return input;
    };

    let draft_width = tag_draft.chars().count() as f32 * 7.0;
    let suffix_width = (suffix.chars().count() as f32 * 7.0 + 6.0)
        .max(14.0)
        .min((width - 8.0).max(1.0));
    let suffix_x = (8.0 + draft_width + 2.0).min((width - suffix_width).max(8.0));
    ui::stack([
        input,
        ui::floating_layer(
            ui::Point::new(suffix_x, 1.0),
            ui::Vector2::new(suffix_width, TAG_FIELD_CONTROL_HEIGHT - 3.0),
            ui::text(suffix.to_string())
                .text_background(ui::TextBackgroundRole::Accent)
                .on_accent_text()
                .text_inset(3.0, 0.0)
                .key("metadata-tag-completion-ghost")
                .width(suffix_width)
                .height(TAG_FIELD_CONTROL_HEIGHT - 3.0),
        )
        .key("metadata-tag-completion-ghost-layer")
        .fill(),
    ])
    .width(width)
    .height(TAG_FIELD_CONTROL_HEIGHT)
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
    let mut badge = ui::badge(tag.to_string())
        .message(GuiMessage::SelectMetadataTag(tag.to_string()))
        .key(format!("metadata-tag-accepted-{tag}"))
        .style(style)
        .sizing(ui::WidgetSizing::fixed(ui::Vector2::new(
            tag_pill_width(tag),
            TAG_FIELD_CONTROL_HEIGHT,
        )))
        .height(TAG_FIELD_CONTROL_HEIGHT)
        .width(tag_pill_width(tag));
    if !selected && category_id != "playback-type" {
        badge = badge.subtle();
    }
    badge
}

fn metadata_tag_category_style(category_id: &str, selected: bool) -> WidgetStyle {
    WidgetStyle {
        tone: match category_id {
            "playback-type" => WidgetTone::Warning,
            "sound-type" => WidgetTone::Accent,
            "character" => WidgetTone::Success,
            "prefix" => WidgetTone::Danger,
            "tuning-scale" => WidgetTone::Neutral,
            _ => WidgetTone::Neutral,
        },
        prominence: if selected || category_id == "playback-type" {
            ui::WidgetProminence::Strong
        } else {
            ui::WidgetProminence::Subtle
        },
    }
}

fn pending_category_tag_token(tag: &str) -> ui::View<GuiMessage> {
    ui::badge(tag.to_string())
        .subtle()
        .message(GuiMessage::Noop)
        .key(format!("metadata-tag-pending-category-{tag}"))
        .style(WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .sizing(ui::WidgetSizing::fixed(ui::Vector2::new(
            tag_pill_width(tag),
            TAG_FIELD_CONTROL_HEIGHT,
        )))
        .height(TAG_FIELD_CONTROL_HEIGHT)
        .width(tag_pill_width(tag))
}

fn metadata_sidebar_panel(content: ui::View<GuiMessage>, height: f32) -> ui::View<GuiMessage> {
    content
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Subtle,
        })
        .padding(6.0)
        .fill_width()
        .height(height)
}
