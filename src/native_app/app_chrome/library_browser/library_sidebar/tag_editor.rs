use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::TagEditorViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    SIDEBAR_PANEL_HEADER_CONTENT_SPACING, SIDEBAR_PANEL_HEADER_HEIGHT,
};

use super::tag_entry_layout::TAG_FIELD_LINE_GAP;

mod identity;
mod projection;
mod rows;

use projection::{TagEditorFieldProjection, TagEditorProjection};
use rows::tag_entry_row;

const METADATA_PANEL_PADDING: f32 = 6.0;
const METADATA_PANEL_HEADER_CONTENT_SPACING: f32 = SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
const METADATA_TAG_LIBRARY_TOGGLE_WIDTH: f32 = 22.0;

pub(super) fn tag_editor_section(
    model: &TagEditorViewModel,
    sidebar_width: f32,
    panel_height: f32,
) -> ui::View<GuiMessage> {
    let projection =
        TagEditorProjection::from_model(model, sidebar_width, METADATA_TAG_LIBRARY_TOGGLE_WIDTH);
    metadata_section(projection, panel_height)
}

fn metadata_section(projection: TagEditorProjection, panel_height: f32) -> ui::View<GuiMessage> {
    let Some(field) = projection.field else {
        return metadata_sidebar_panel(ui::empty().fill_width().fill_height(), panel_height);
    };
    let field_height = field.layout.field_height;

    metadata_sidebar_panel(
        tag_entry_field(field)
            .key(identity::TAG_ENTRY_FIELD_KEY)
            .fill_width()
            .height(field_height),
        panel_height,
    )
}

fn tag_entry_field(field: TagEditorFieldProjection) -> ui::View<GuiMessage> {
    let content = ui::column(
        field
            .rows
            .into_iter()
            .enumerate()
            .map(|(row_index, row)| tag_entry_row(row, row_index))
            .collect::<Vec<_>>(),
    )
    .fill_width()
    .height(field.layout.content_height)
    .spacing(TAG_FIELD_LINE_GAP);

    if field.layout.requires_scroll {
        ui::scroll(content)
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
            .padding(3.0)
            .fill_width()
            .height(field.layout.field_height)
    } else {
        content.fill_width().height(field.layout.field_height)
    }
}

fn metadata_sidebar_panel(content: ui::View<GuiMessage>, height: f32) -> ui::View<GuiMessage> {
    let panel = ui::panel_section_from_header_parts(
        radiant::application::PanelSectionHeaderParts::resize_header(
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
