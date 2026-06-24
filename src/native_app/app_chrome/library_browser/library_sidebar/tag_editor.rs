use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::TagEditorViewModel;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    SIDEBAR_PANEL_HEADER_CONTENT_SPACING, SIDEBAR_PANEL_HEADER_HEIGHT,
};

use super::tag_entry_layout::{
    TAG_FIELD_LINE_GAP, TagEntryFieldInput, TagEntryFieldProjection, tag_field_content_width,
};

mod identity;
mod rows;

use rows::{TagEntryRowContext, tag_entry_row};

const METADATA_PANEL_PADDING: f32 = 6.0;
const METADATA_PANEL_HEADER_CONTENT_SPACING: f32 = SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
const METADATA_TAG_LIBRARY_TOGGLE_WIDTH: f32 = 22.0;

struct TagEditorFieldParts<'a> {
    draft: &'a str,
    tokens: &'a [String],
    pending_category_tag: Option<&'a str>,
    input_placeholder: &'a str,
    completion_suffix: Option<&'a str>,
    tags: &'a [String],
    mixed_tags: &'a [String],
    display_categories: &'a [MetadataTagDisplayCategory],
    selected_tag: Option<&'a str>,
    content_width: f32,
}

impl<'a> TagEditorFieldParts<'a> {
    fn from_model(model: &'a TagEditorViewModel, content_width: f32) -> Self {
        Self {
            draft: model.draft.as_str(),
            tokens: model.tokens.as_slice(),
            pending_category_tag: model.pending_category_tag.as_deref(),
            input_placeholder: model.input_placeholder.as_str(),
            completion_suffix: model.completion_suffix.as_deref(),
            tags: model.tags.as_slice(),
            mixed_tags: model.mixed_tags.as_slice(),
            display_categories: model.display_categories.as_slice(),
            selected_tag: model.selected_tag.as_deref(),
            content_width,
        }
    }

    fn projection_input(&self) -> TagEntryFieldInput<'_> {
        TagEntryFieldInput {
            draft: self.draft,
            tokens: self.tokens,
            pending_category_tag: self.pending_category_tag,
            placeholder: self.input_placeholder,
            completion_suffix: self.completion_suffix,
            tags: self.tags,
            display_categories: self.display_categories,
            content_width: self.content_width,
            library_toggle_width: Some(METADATA_TAG_LIBRARY_TOGGLE_WIDTH),
        }
    }
}

pub(super) fn tag_editor_section(
    model: &TagEditorViewModel,
    sidebar_width: f32,
    panel_height: f32,
) -> ui::View<GuiMessage> {
    let content_width = tag_field_content_width(sidebar_width);
    let field = TagEditorFieldParts::from_model(model, content_width);
    let field_height = tag_field_height(&field);
    metadata_section(&field, field_height, panel_height, model.has_selected_file)
}

fn metadata_section(
    field: &TagEditorFieldParts<'_>,
    tag_field_height: f32,
    panel_height: f32,
    has_selected_file: bool,
) -> ui::View<GuiMessage> {
    if !has_selected_file {
        return metadata_sidebar_panel(ui::empty().fill_width().fill_height(), None, panel_height);
    }

    metadata_sidebar_panel(
        tag_entry_field(field, tag_field_height)
            .key(identity::TAG_ENTRY_FIELD_KEY)
            .fill_width()
            .height(tag_field_height),
        Some(field.tags.len()),
        panel_height,
    )
}

fn tag_entry_field(field: &TagEditorFieldParts<'_>, height: f32) -> ui::View<GuiMessage> {
    let projection = TagEntryFieldProjection::from_input(field.projection_input());
    let row_context = TagEntryRowContext::from_field(field);
    let content = ui::column(
        projection
            .rows
            .into_iter()
            .enumerate()
            .map(|(row_index, row)| tag_entry_row(row, &row_context, row_index))
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

fn tag_field_height(field: &TagEditorFieldParts<'_>) -> f32 {
    TagEntryFieldProjection::from_input(field.projection_input())
        .layout
        .field_height
}

fn metadata_sidebar_panel(
    content: ui::View<GuiMessage>,
    _tag_count: Option<usize>,
    height: f32,
) -> ui::View<GuiMessage> {
    let panel = ui::panel_section_from_header_parts(
        ui::PanelSectionHeaderParts::resize_header(
            "metadata-resize-header",
            SIDEBAR_PANEL_HEADER_HEIGHT,
            content,
            |message| GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeMetadataPanel(message)),
        )
        .header_id(identity::metadata_resize_header_id())
        .height(height)
        .padding(METADATA_PANEL_PADDING)
        .spacing(METADATA_PANEL_HEADER_CONTENT_SPACING),
    )
    .fill_width();
    #[cfg(test)]
    {
        panel.id(identity::metadata_sidebar_panel_id())
    }
    #[cfg(not(test))]
    {
        panel
    }
}
