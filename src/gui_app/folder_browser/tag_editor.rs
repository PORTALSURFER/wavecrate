use radiant::prelude as ui;
use radiant::widgets::DragHandleMessage;

use crate::gui_app::metadata_tags::MetadataTagDisplayCategory;
use crate::gui_app::metadata_tags::{metadata_tag_category_is_pinned, metadata_tag_category_style};

use super::tag_entry_layout::{
    TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_ITEM_GAP, TAG_FIELD_LINE_GAP, TagEntryRowItem,
    metadata_tag_category_id_for_display, order_metadata_tags_for_display, rows_height,
    tag_field_layout, tag_field_rows, tag_input_display_value, tag_input_width,
    tag_input_width_for_placeholder, tag_pill_width,
};
use super::{FolderBrowserMessage, FolderBrowserState, GuiMessage};

const METADATA_TAG_INPUT_ID: u64 = 0x5743_0000_0000_5447;
#[cfg(test)]
pub(in crate::gui_app) const METADATA_SIDEBAR_PANEL_ID: u64 = 0x5743_0000_0000_5448;
const MAX_METADATA_PANEL_HEIGHT: f32 = 240.0;
const METADATA_PANEL_PADDING: f32 = 6.0;
const METADATA_PANEL_TITLE_HEIGHT: f32 = 20.0;
const METADATA_PANEL_HEADER_CONTENT_SPACING: f32 = 4.0;
pub(in crate::gui_app) const COLLAPSED_METADATA_PANEL_HEIGHT: f32 =
    METADATA_PANEL_PADDING * 2.0 + METADATA_PANEL_TITLE_HEIGHT;
const MIN_METADATA_PANEL_HEIGHT: f32 = COLLAPSED_METADATA_PANEL_HEIGHT;

impl FolderBrowserState {
    pub(in crate::gui_app) fn metadata_panel_height(&self) -> f32 {
        self.metadata_panel_height
    }

    pub(super) fn resize_metadata_panel(&mut self, message: DragHandleMessage) {
        if let Some(height) = ui::update_collapsible_panel_resize_drag(
            &mut self.metadata_panel_resize,
            message,
            ui::PanelResizeEdge::Top,
            self.metadata_panel_height,
            MIN_METADATA_PANEL_HEIGHT,
            MAX_METADATA_PANEL_HEIGHT,
            COLLAPSED_METADATA_PANEL_HEIGHT,
        ) {
            self.metadata_panel_height = height;
        }
    }
}

pub(super) fn metadata_section(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_pending_category_tag: Option<&str>,
    tag_input_placeholder: &str,
    tag_completion_suffix: Option<&str>,
    tags: &[String],
    tag_display_categories: &[MetadataTagDisplayCategory],
    selected_metadata_tag: Option<&str>,
    tag_field_content_width: f32,
    tag_field_height: f32,
    panel_height: f32,
    has_selected_file: bool,
) -> ui::View<GuiMessage> {
    if !has_selected_file {
        return metadata_sidebar_panel(ui::empty().fill_width().fill_height(), panel_height);
    }

    metadata_sidebar_panel(
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
        .fill_height()
        .spacing(3.0),
        panel_height,
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
    let field_layout = tag_field_layout(row_count, content_width);
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

    if field_layout.requires_scroll {
        ui::scroll(content)
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
            .padding(3.0)
            .fill_width()
            .height(height)
    } else {
        content.fill_width().height(height)
    }
}

pub(in crate::gui_app) fn tag_field_height(
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
    tag_field_layout(rows, content_width).field_height
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
    let panel = ui::panel_section_from_parts(
        ui::PanelSectionParts::new("Metadata", content)
            .trailing(
                ui::drag_handle_mapped(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeMetadataPanel(message))
                })
                .key("metadata-resize-handle")
                .size(26.0, 18.0),
            )
            .padding(METADATA_PANEL_PADDING)
            .spacing(METADATA_PANEL_HEADER_CONTENT_SPACING)
            .title_height(METADATA_PANEL_TITLE_HEIGHT)
            .height(height),
    )
    .fill_width();
    #[cfg(test)]
    {
        panel.id(METADATA_SIDEBAR_PANEL_ID)
    }
    #[cfg(not(test))]
    {
        panel
    }
}
