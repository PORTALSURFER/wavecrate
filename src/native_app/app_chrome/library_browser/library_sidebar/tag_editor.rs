use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::TagEditorViewModel;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::metadata::{metadata_tag_category_is_pinned, metadata_tag_category_style};
use crate::native_app::sample_library::folder_browser::FolderBrowserMessage;
use crate::native_app::ui::ids as widget_ids;

use super::tag_entry_layout::{
    TAG_FIELD_CONTROL_HEIGHT, TAG_FIELD_ITEM_GAP, TAG_FIELD_LINE_GAP, TagEntryFieldProjection,
    TagEntryRowItem, metadata_tag_category_id_for_display, tag_field_content_width, tag_pill_width,
};

#[cfg(test)]
pub(in crate::native_app) const METADATA_TAG_INPUT_ID: u64 = widget_ids::METADATA_TAG_INPUT_ID;
#[cfg(not(test))]
const METADATA_TAG_INPUT_ID: u64 = widget_ids::METADATA_TAG_INPUT_ID;
#[cfg(test)]
pub(in crate::native_app) const METADATA_SIDEBAR_PANEL_ID: u64 =
    widget_ids::METADATA_SIDEBAR_PANEL_ID;
#[cfg(test)]
pub(in crate::native_app) const METADATA_TAG_LIBRARY_TOGGLE_ID: u64 =
    widget_ids::METADATA_TAG_LIBRARY_TOGGLE_ID;
const METADATA_PANEL_PADDING: f32 = 6.0;
const METADATA_PANEL_TITLE_HEIGHT: f32 = 20.0;
const METADATA_PANEL_HEADER_CONTENT_SPACING: f32 = 4.0;
const METADATA_HEADER_TRAILING_HEIGHT: f32 = 20.0;
const METADATA_HEADER_RESIZE_HANDLE_WIDTH: f32 = 26.0;
const METADATA_HEADER_RESIZE_HANDLE_HEIGHT: f32 = 18.0;

pub(super) fn tag_editor_section(
    model: &TagEditorViewModel,
    sidebar_width: f32,
    panel_height: f32,
) -> ui::View<GuiMessage> {
    let content_width = tag_field_content_width(sidebar_width);
    let field_height = tag_field_height(
        model.draft.as_str(),
        model.tokens.as_slice(),
        model.pending_category_tag.as_deref(),
        model.input_placeholder.as_str(),
        model.completion_suffix.as_deref(),
        model.tags.as_slice(),
        model.display_categories.as_slice(),
        content_width,
    );
    metadata_section(
        model.draft.as_str(),
        model.tokens.as_slice(),
        model.pending_category_tag.as_deref(),
        model.input_placeholder.as_str(),
        model.completion_suffix.as_deref(),
        model.tags.as_slice(),
        model.display_categories.as_slice(),
        model.selected_tag.as_deref(),
        content_width,
        field_height,
        panel_height,
        model.has_selected_file,
    )
}

fn metadata_section(
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
    let projection = TagEntryFieldProjection::new(
        tag_draft,
        tag_tokens,
        tag_pending_category_tag,
        tag_input_placeholder,
        tag_completion_suffix,
        tags,
        tag_display_categories,
        content_width,
    );
    let content = ui::column(
        projection
            .rows
            .into_iter()
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
    .height(projection.layout.content_height)
    .spacing(TAG_FIELD_LINE_GAP);

    if projection.layout.requires_scroll {
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
    TagEntryFieldProjection::new(
        tag_draft,
        tag_tokens,
        tag_pending_category_tag,
        tag_input_placeholder,
        tag_completion_suffix,
        tags,
        tag_display_categories,
        content_width,
    )
    .layout
    .field_height
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
