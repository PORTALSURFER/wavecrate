use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::FilterSectionViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    SIDEBAR_PANEL_HEADER_CONTENT_SPACING, SIDEBAR_PANEL_HEADER_HEIGHT,
};
use crate::native_app::ui::ids as widget_ids;

mod rows;
#[cfg(test)]
mod tests;

use rows::{FILTER_ROW_HEIGHT, FILTER_ROW_SPACING, filter_rows};

const FILTER_PANEL_PADDING: f32 = 6.0;
#[cfg(test)]
const FILTER_PANEL_HEADER_HEIGHT: f32 = SIDEBAR_PANEL_HEADER_HEIGHT;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
const FILTER_SECTION_SCROLL_NODE_ID: u64 = widget_ids::FILTER_SECTION_SCROLL_NODE_ID;
const FILTER_RESIZE_HEADER_ID: u64 = widget_ids::FILTER_RESIZE_HEADER_ID;

#[cfg(test)]
const FILTER_SECTION_NODE_ID: u64 = widget_ids::FILTER_SECTION_NODE_ID;

pub(super) fn filter_section(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let panel = ui::panel_section_from_header_parts(
        ui::PanelSectionHeaderParts::resize_header(
            "filter-resize-header",
            SIDEBAR_PANEL_HEADER_HEIGHT,
            filter_controls(model),
            |message| GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFilterPanel(message)),
        )
        .header_id(FILTER_RESIZE_HEADER_ID)
        .height(model.panel_height)
        .padding(FILTER_PANEL_PADDING)
        .spacing(FILTER_PANEL_HEADER_CONTENT_SPACING),
    )
    .fill_width();

    #[cfg(test)]
    {
        panel.id(FILTER_SECTION_NODE_ID)
    }
    #[cfg(not(test))]
    {
        panel
    }
}

fn filter_controls(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let rows = filter_rows(model);
    let content_height = filter_controls_content_height(rows.len());

    ui::scroll(
        ui::column(rows)
            .fill_width()
            .height(content_height)
            .spacing(FILTER_ROW_SPACING),
    )
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
    .id(FILTER_SECTION_SCROLL_NODE_ID)
    .fill_width()
    .fill_height()
}

fn filter_controls_content_height(row_count: usize) -> f32 {
    FILTER_ROW_HEIGHT * row_count as f32 + FILTER_ROW_SPACING * row_count.saturating_sub(1) as f32
}
