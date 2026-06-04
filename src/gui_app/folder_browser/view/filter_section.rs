use radiant::{prelude as ui, widgets::TextInputMessageKind};
use std::collections::HashSet;

use super::super::{FolderBrowserMessage, FolderBrowserState, GuiMessage};

const FILTER_PANEL_PADDING: f32 = 6.0;
const FILTER_PANEL_HEADER_HEIGHT: f32 = 20.0;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = 4.0;
const MAX_FILTER_PANEL_HEIGHT: f32 = 180.0;
pub(in crate::gui_app::folder_browser) const COLLAPSED_FILTER_PANEL_HEIGHT: f32 =
    FILTER_PANEL_PADDING * 2.0 + FILTER_PANEL_HEADER_HEIGHT;
const MIN_FILTER_PANEL_HEIGHT: f32 = COLLAPSED_FILTER_PANEL_HEIGHT;
pub(in crate::gui_app::folder_browser) const DEFAULT_FILTER_PANEL_HEIGHT: f32 = 76.0;
const FILTER_ROW_HEIGHT: f32 = 24.0;
const FILTER_ROW_LABEL_WIDTH: f32 = 112.0;
const FILTER_ROW_HORIZONTAL_PADDING: f32 = 6.0;
const FILTER_ROW_VERTICAL_PADDING: f32 = 1.0;
const FILTER_ROW_SPACING: f32 = 6.0;
const NAME_FILTER_INPUT_ID: u64 = 0x5743_0000_0000_4602;

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

    pub(in crate::gui_app) fn name_filter(&self) -> &str {
        self.name_filter.as_str()
    }

    pub(in crate::gui_app) fn apply_name_filter_input(
        &mut self,
        message: radiant::widgets::TextInputMessage,
    ) {
        if message.kind() == TextInputMessageKind::CompletionRequested {
            return;
        }
        let value = message.into_value();
        if self.name_filter == value {
            return;
        }
        self.name_filter = value;
        self.retain_visible_file_selection_after_filter();
        self.reset_file_view();
    }

    fn retain_visible_file_selection_after_filter(&mut self) {
        let visible_ids = self
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<HashSet<_>>();
        self.selected_file_ids.retain(|id| visible_ids.contains(id));
        if self
            .selected_file
            .as_ref()
            .is_some_and(|id| !visible_ids.contains(id))
        {
            self.selected_file = None;
        }
    }
}

pub(super) fn filter_section(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let panel = ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Filter",
            ui::column([name_filter_row(state), type_filter_row()])
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

fn name_filter_row(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    filter_row(
        "name",
        ui::text("Name")
            .key("filter-name-label")
            .size(FILTER_ROW_LABEL_WIDTH, 20.0),
        ui::text_input(state.name_filter().to_owned())
            .placeholder("Any")
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::NameFilterInput(message))
            })
            .id(NAME_FILTER_INPUT_ID)
            .key("filter-name-input")
            .fill_width()
            .height(20.0),
    )
}

fn type_filter_row() -> ui::View<GuiMessage> {
    filter_row(
        "type",
        ui::text("Type")
            .key("filter-type-label")
            .size(FILTER_ROW_LABEL_WIDTH, 20.0),
        ui::text("Audio")
            .key("filter-type-value")
            .fill_width()
            .height(20.0),
    )
}

fn filter_row(
    id: &'static str,
    label: ui::View<GuiMessage>,
    value: ui::View<GuiMessage>,
) -> ui::View<GuiMessage> {
    ui::row([label, value])
        .key(format!("filter-row-{id}"))
        .fill_width()
        .height(FILTER_ROW_HEIGHT)
        .padding_x(FILTER_ROW_HORIZONTAL_PADDING)
        .padding_y(FILTER_ROW_VERTICAL_PADDING)
        .spacing(FILTER_ROW_SPACING)
        .hoverable()
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

    #[test]
    fn filter_section_projects_name_text_input() {
        let state = FolderBrowserState::load_default();

        let frame = filter_section(&state)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, 76.0));
        let input = frame
            .paint_plan
            .first_text_input()
            .expect("name filter should project a text input");

        assert_eq!(input.widget_id, NAME_FILTER_INPUT_ID);
        assert_eq!(input.state.value, "");
        assert_eq!(
            input.placeholder.as_ref().map(|value| value.as_str()),
            Some("Any")
        );
        assert!(
            !frame
                .paint_plan
                .contains_text_after_x("Any", input.rect.min.x),
            "name filter should not paint Any as a read-only property value"
        );
    }
}
