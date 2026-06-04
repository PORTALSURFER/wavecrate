use radiant::prelude as ui;

use super::super::{FolderBrowserMessage, FolderBrowserState, GuiMessage};

const FILTER_PANEL_PADDING: f32 = 6.0;
const FILTER_PANEL_HEADER_HEIGHT: f32 = 20.0;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = 4.0;
const MAX_FILTER_PANEL_HEIGHT: f32 = 180.0;
pub(in crate::gui_app::folder_browser) const COLLAPSED_FILTER_PANEL_HEIGHT: f32 =
    FILTER_PANEL_PADDING * 2.0 + FILTER_PANEL_HEADER_HEIGHT;
const MIN_FILTER_PANEL_HEIGHT: f32 = COLLAPSED_FILTER_PANEL_HEIGHT;
pub(in crate::gui_app::folder_browser) const DEFAULT_FILTER_PANEL_HEIGHT: f32 = 76.0;

#[cfg(test)]
const FILTER_SECTION_NODE_ID: u64 = 0x5743_0000_0000_4601;

impl FolderBrowserState {
    pub(in crate::gui_app) fn filter_panel_height(&self) -> f32 {
        self.filter_panel.size()
    }

    pub(in crate::gui_app::folder_browser) fn resize_filter_panel(
        &mut self,
        message: ui::DragHandleMessage,
    ) {
        self.filter_panel.resize_collapsible(
            message,
            ui::CollapsiblePanelResizeConstraints::new(
                ui::PanelResizeEdge::Top,
                MIN_FILTER_PANEL_HEIGHT,
                MAX_FILTER_PANEL_HEIGHT,
                COLLAPSED_FILTER_PANEL_HEIGHT,
            ),
        );
    }
}

pub(super) fn filter_section(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let panel = ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Filter",
            ui::property_rows([
                ui::PropertyRow::new("name", "Name", "Any"),
                ui::PropertyRow::new("type", "Type", "Audio"),
            ]),
        )
        .trailing_resize_handle("filter-resize-handle", |message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFilterPanel(message))
        })
        .padding(FILTER_PANEL_PADDING)
        .spacing(FILTER_PANEL_HEADER_CONTENT_SPACING)
        .title_height(FILTER_PANEL_HEADER_HEIGHT)
        .height(state.filter_panel_height()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn filter_section_layout_uses_configured_height() {
        let mut state = FolderBrowserState::load_default();
        state.resize_filter_panel(ui::DragHandleMessage::started(ui::Point::new(0.0, 200.0)));
        state.resize_filter_panel(ui::DragHandleMessage::moved(ui::Point::new(0.0, 120.0)));

        let layout = ui::column([
            filter_section(&state),
            ui::spacer().fill_width().fill_height(),
        ])
        .view_layout_at_size(ui::Vector2::new(240.0, 600.0));
        let section = layout
            .rects
            .get(&FILTER_SECTION_NODE_ID)
            .expect("filter section layout rect");

        assert_eq!(section.height(), state.filter_panel_height());
    }
}
