use radiant::{prelude as ui, widgets::DragHandleMessage};

use super::super::FolderBrowserState;

pub(in crate::native_app) const COLLECTION_ROW_HEIGHT: f32 = 22.0;
pub(in crate::native_app) const COLLECTION_ROW_SPACING: f32 = 1.0;
pub(in crate::native_app) const COLLECTIONS_PANEL_PADDING: f32 = 6.0;
pub(in crate::native_app) const COLLECTIONS_PANEL_HEADER_HEIGHT: f32 =
    super::super::SIDEBAR_PANEL_HEADER_HEIGHT;
pub(in crate::native_app) const COLLECTIONS_PANEL_HEADER_CONTENT_SPACING: f32 =
    super::super::SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
pub(in crate::native_app) const COLLECTIONS_LIST_SCROLL_CHROME: f32 = 8.0;
pub(in crate::native_app) const COLLAPSED_COLLECTIONS_PANEL_HEIGHT: f32 =
    COLLECTIONS_PANEL_PADDING * 2.0 + COLLECTIONS_PANEL_HEADER_HEIGHT;
pub(in crate::native_app) const MIN_COLLECTIONS_PANEL_HEIGHT: f32 =
    COLLAPSED_COLLECTIONS_PANEL_HEIGHT;
pub(in crate::native_app) const DEFAULT_COLLECTIONS_PANEL_HEIGHT: f32 = 130.0;

impl FolderBrowserState {
    pub(in crate::native_app) fn collections_panel_height(&self) -> f32 {
        self.panel_layout.collections.size()
    }

    pub(in crate::native_app) fn collections_list_height(&self) -> f32 {
        ui::fixed_row_stack_height(
            self.collection_panel.collections.len(),
            COLLECTION_ROW_HEIGHT,
            COLLECTION_ROW_SPACING,
        )
    }

    pub(in crate::native_app) fn max_collections_panel_height(&self) -> f32 {
        useful_collections_panel_height(self.collection_panel.collections.len())
    }

    pub(in crate::native_app) fn resize_collections_panel(&mut self, message: DragHandleMessage) {
        self.panel_layout.collections.resize_collapsible(
            message,
            ui::CollapsiblePanelResizeConstraints::top(
                MIN_COLLECTIONS_PANEL_HEIGHT,
                self.max_collections_panel_height(),
                COLLAPSED_COLLECTIONS_PANEL_HEIGHT,
            ),
        );
    }
}

fn useful_collections_panel_height(row_count: usize) -> f32 {
    collections_panel_geometry().section_height_for_content_height(
        COLLECTIONS_LIST_SCROLL_CHROME
            + ui::fixed_row_stack_height(row_count, COLLECTION_ROW_HEIGHT, COLLECTION_ROW_SPACING),
    )
}

fn collections_panel_geometry() -> ui::PanelSectionGeometry {
    ui::PanelSectionGeometry::new()
        .padding(COLLECTIONS_PANEL_PADDING)
        .spacing(COLLECTIONS_PANEL_HEADER_CONTENT_SPACING)
        .title_height(COLLECTIONS_PANEL_HEADER_HEIGHT)
}
