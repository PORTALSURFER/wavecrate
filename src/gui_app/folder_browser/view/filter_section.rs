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
            ui::column([
                filter_row("name", "Name", "Any"),
                filter_row("type", "Type", "Audio"),
            ])
            .fill_width()
            .spacing(1.0),
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

fn filter_row(id: &str, label: &str, value: &str) -> ui::View<GuiMessage> {
    ui::row([
        ui::text(label.to_string())
            .key(format!("filter-{id}-label"))
            .size(112.0, 20.0),
        ui::text(value.to_string())
            .key(format!("filter-{id}-value"))
            .fill_width()
            .height(20.0),
    ])
    .key(format!("filter-row-{id}"))
    .fill_width()
    .height(24.0)
    .padding_x(6.0)
    .padding_y(1.0)
    .spacing(6.0)
    .hoverable()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn filter_section_layout_uses_configured_height() {
        let mut state = FolderBrowserState::load_default();
        state.resize_filter_panel(ui::DragHandleMessage::Started {
            position: ui::Point::new(0.0, 200.0),
        });
        state.resize_filter_panel(ui::DragHandleMessage::Moved {
            position: ui::Point::new(0.0, 120.0),
        });

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
