use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::CollectionsSectionViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    COLLECTION_ROW_SPACING, COLLECTIONS_PANEL_HEADER_CONTENT_SPACING, COLLECTIONS_PANEL_PADDING,
    SIDEBAR_PANEL_HEADER_HEIGHT,
};
use crate::native_app::ui::ids as widget_ids;

mod identity;
mod rows;
#[cfg(test)]
mod tests;

use super::edge_aligned_resize_panel;
use rows::collection_row;

/// Stable layout node id for collection-panel resize regression coverage.
const COLLECTIONS_SECTION_NODE_ID: u64 = widget_ids::COLLECTIONS_SECTION_NODE_ID;
/// Stable layout node id for the collection rows scroll viewport.
const COLLECTIONS_LIST_SCROLL_NODE_ID: u64 = widget_ids::COLLECTIONS_LIST_SCROLL_NODE_ID;
const COLLECTIONS_RESIZE_HEADER_ID: u64 = widget_ids::COLLECTIONS_RESIZE_HEADER_ID;

pub(super) fn collections_section(model: &CollectionsSectionViewModel) -> ui::View<GuiMessage> {
    let rows = model.rows.iter().map(collection_row).collect::<Vec<_>>();
    edge_aligned_resize_panel(
        "collections-resize-header",
        COLLECTIONS_RESIZE_HEADER_ID,
        SIDEBAR_PANEL_HEADER_HEIGHT,
        ui::scroll(
            ui::column(rows)
                .spacing(COLLECTION_ROW_SPACING)
                .fill_width()
                .height(model.list_height),
        )
        .id(COLLECTIONS_LIST_SCROLL_NODE_ID)
        .fill_width()
        .fill_height(),
        model.panel_height,
        COLLECTIONS_PANEL_PADDING,
        COLLECTIONS_PANEL_HEADER_CONTENT_SPACING,
        |message| GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeCollectionsPanel(message)),
    )
    .id(COLLECTIONS_SECTION_NODE_ID)
}
