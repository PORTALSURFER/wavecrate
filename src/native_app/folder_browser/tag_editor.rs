use radiant::prelude as ui;
use radiant::widgets::DragHandleMessage;

use crate::native_app::metadata_tags::MetadataTagDisplayCategory;
use crate::native_app::metadata_tags::{
    metadata_tag_category_is_pinned, metadata_tag_category_style,
};

use super::tag_entry_layout::{
    TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_ITEM_GAP, TAG_FIELD_LINE_GAP, TagEntryRowItem,
    metadata_tag_category_id_for_display, order_metadata_tags_for_display, tag_field_layout,
    tag_field_rows, tag_input_width_with_completion,
    tag_input_width_with_completion_or_placeholder, tag_pill_width,
};
use super::{FolderBrowserMessage, FolderBrowserState, GuiMessage};

#[cfg(test)]
pub(in crate::native_app) const METADATA_TAG_INPUT_ID: u64 = 0x5743_0000_0000_5447;
#[cfg(not(test))]
const METADATA_TAG_INPUT_ID: u64 = 0x5743_0000_0000_5447;
#[cfg(test)]
pub(in crate::native_app) const METADATA_SIDEBAR_PANEL_ID: u64 = 0x5743_0000_0000_5448;
#[cfg(test)]
pub(in crate::native_app) const METADATA_TAG_LIBRARY_TOGGLE_ID: u64 = 0x5743_0000_0000_5449;
const MAX_METADATA_PANEL_HEIGHT: f32 = 240.0;
const METADATA_PANEL_PADDING: f32 = 6.0;
const METADATA_PANEL_TITLE_HEIGHT: f32 = 20.0;
const METADATA_PANEL_HEADER_CONTENT_SPACING: f32 = 4.0;
const METADATA_HEADER_TRAILING_HEIGHT: f32 = 20.0;
const METADATA_HEADER_RESIZE_HANDLE_WIDTH: f32 = 26.0;
const METADATA_HEADER_RESIZE_HANDLE_HEIGHT: f32 = 18.0;
pub(in crate::native_app) const COLLAPSED_METADATA_PANEL_HEIGHT: f32 =
    METADATA_PANEL_PADDING * 2.0 + METADATA_PANEL_TITLE_HEIGHT;
const MIN_METADATA_PANEL_HEIGHT: f32 = COLLAPSED_METADATA_PANEL_HEIGHT;

impl FolderBrowserState {
    pub(in crate::native_app) fn metadata_panel_height(&self) -> f32 {
        self.metadata_panel.size()
    }

    pub(super) fn resize_metadata_panel(&mut self, message: DragHandleMessage) {
        self.metadata_panel.resize_collapsible(
            message,
            ui::CollapsiblePanelResizeConstraints::top(
                MIN_METADATA_PANEL_HEIGHT,
                MAX_METADATA_PANEL_HEIGHT,
                COLLAPSED_METADATA_PANEL_HEIGHT,
            ),
        );
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
        return metadata_sidebar_panel(ui::empty().fill_width().fill_height(), None, panel_height);
    }

    metadata_sidebar_panel(
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
        Some(tags.len()),
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
    let input_width = if pending_category_tag.is_some() {
        tag_input_width_with_completion(tag_draft, tag_completion_suffix)
    } else {
        tag_input_width_with_completion_or_placeholder(
            tag_draft,
            tag_completion_suffix,
            tag_input_placeholder,
        )
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
    .height(field_layout.content_height)
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

pub(in crate::native_app) fn tag_field_height(
    tag_draft: &str,
    tag_tokens: &[String],
    tag_pending_category_tag: Option<&str>,
    tag_input_placeholder: &str,
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
    let input_width = if tag_pending_category_tag.is_some() {
        tag_input_width_with_completion(tag_draft, tag_completion_suffix)
    } else {
        tag_input_width_with_completion_or_placeholder(
            tag_draft,
            tag_completion_suffix,
            tag_input_placeholder,
        )
    };
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

pub(in crate::native_app) fn metadata_tag_completion_bottom_inset(panel_height: f32) -> f32 {
    metadata_sidebar_panel_parts(ui::empty(), None, panel_height)
        .content_top_inset_from_bottom(panel_height)
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
        .actions(ui::InteractiveRowActions::new().activate_secondary_key(
            tag_for_input,
            GuiMessage::SelectMetadataTag,
            |tag, position| GuiMessage::OpenMetadataTagContextMenu { tag, position },
        ))
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

fn metadata_sidebar_panel(
    content: ui::View<GuiMessage>,
    tag_count: Option<usize>,
    height: f32,
) -> ui::View<GuiMessage> {
    let panel =
        ui::panel_section_from_parts(metadata_sidebar_panel_parts(content, tag_count, height))
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

fn metadata_sidebar_panel_parts(
    content: ui::View<GuiMessage>,
    tag_count: Option<usize>,
    height: f32,
) -> ui::PanelSectionParts<GuiMessage> {
    ui::PanelSectionParts::new("Tags", content)
        .trailing(metadata_header_trailing(tag_count))
        .padding(METADATA_PANEL_PADDING)
        .spacing(METADATA_PANEL_HEADER_CONTENT_SPACING)
        .title_height(METADATA_PANEL_TITLE_HEIGHT)
        .height(height)
}

fn metadata_header_trailing(tag_count: Option<usize>) -> ui::View<GuiMessage> {
    let mut controls = Vec::new();
    if let Some(count) = tag_count {
        controls.push(
            ui::text(format!("({count})"))
                .height(METADATA_HEADER_TRAILING_HEIGHT)
                .width(32.0),
        );
        controls.push(metadata_tag_library_toggle());
    }
    controls.push(
        ui::drag_handle_mapped(|message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeMetadataPanel(message))
        })
        .key("metadata-resize-handle")
        .size(
            METADATA_HEADER_RESIZE_HANDLE_WIDTH,
            METADATA_HEADER_RESIZE_HANDLE_HEIGHT,
        ),
    );
    ui::row(controls)
        .spacing(4.0)
        .height(METADATA_HEADER_TRAILING_HEIGHT)
}

fn metadata_tag_library_toggle() -> ui::View<GuiMessage> {
    let toggle = ui::disclosure_button(false)
        .message(GuiMessage::ToggleMetadataTagLibrary)
        .key("metadata-tag-library-toggle")
        .size(24.0, 20.0);
    #[cfg(test)]
    {
        toggle.id(METADATA_TAG_LIBRARY_TOGGLE_ID)
    }
    #[cfg(not(test))]
    {
        toggle
    }
}
