use radiant::{prelude as ui, widgets::TextInputMessageKind};
use std::collections::{HashMap, HashSet};

use crate::native_app::ui::ids as widget_ids;

use super::super::{FolderBrowserMessage, FolderBrowserState, GuiMessage};

const FILTER_PANEL_PADDING: f32 = 6.0;
const FILTER_PANEL_HEADER_HEIGHT: f32 = 20.0;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = 4.0;
const MAX_FILTER_PANEL_HEIGHT: f32 = 180.0;
pub(in crate::native_app::library_browser::folder_browser) const COLLAPSED_FILTER_PANEL_HEIGHT:
    f32 = FILTER_PANEL_PADDING * 2.0 + FILTER_PANEL_HEADER_HEIGHT;
const MIN_FILTER_PANEL_HEIGHT: f32 = COLLAPSED_FILTER_PANEL_HEIGHT;
pub(in crate::native_app::library_browser::folder_browser) const DEFAULT_FILTER_PANEL_HEIGHT: f32 =
    76.0;
const NAME_FILTER_INPUT_ID: u64 = widget_ids::NAME_FILTER_INPUT_ID;
const TAG_FILTER_INPUT_ID: u64 = widget_ids::TAG_FILTER_INPUT_ID;

#[cfg(test)]
const FILTER_SECTION_NODE_ID: u64 = widget_ids::FILTER_SECTION_NODE_ID;

impl FolderBrowserState {
    pub(in crate::native_app) fn filter_panel_height(&self) -> f32 {
        self.filter_panel.size()
    }

    pub(in crate::native_app::library_browser::folder_browser) fn resize_filter_panel(
        &mut self,
        message: ui::DragHandleMessage,
    ) {
        self.filter_panel.resize_collapsible(
            message,
            ui::CollapsiblePanelResizeConstraints::top(
                MIN_FILTER_PANEL_HEIGHT,
                MAX_FILTER_PANEL_HEIGHT,
                COLLAPSED_FILTER_PANEL_HEIGHT,
            ),
        );
    }

    pub(in crate::native_app) fn name_filter(&self) -> &str {
        self.name_filter.as_str()
    }

    pub(in crate::native_app) fn tag_filter(&self) -> &str {
        self.tag_filter.as_str()
    }

    pub(in crate::native_app) fn apply_name_filter_input(
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

    pub(in crate::native_app) fn apply_tag_filter_input(
        &mut self,
        message: radiant::widgets::TextInputMessage,
    ) {
        if message.kind() == TextInputMessageKind::CompletionRequested {
            return;
        }
        let value = message.into_value();
        if self.tag_filter == value {
            return;
        }
        self.tag_filter = value;
        self.reset_file_view();
    }

    pub(in crate::native_app) fn retain_visible_file_selection_after_tag_filter(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let visible_ids = self
            .selected_audio_files_matching_tags(tags_by_file)
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
        if self.selected_file.is_none() && self.selected_file_ids.is_empty() {
            self.selected_file_ids_explicit = false;
        }
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
        if self.selected_file.is_none() && self.selected_file_ids.is_empty() {
            self.selected_file_ids_explicit = false;
        }
    }
}

pub(super) fn filter_section(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let panel = ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Filter",
            ui::column([name_filter_row(state), tag_filter_row(state)])
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
    ui::form_row(
        "name",
        ui::text("Name").key("filter-name-label"),
        ui::text_input(state.name_filter().to_owned())
            .placeholder("Any")
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::NameFilterInput(message))
            })
            .id(NAME_FILTER_INPUT_ID)
            .key("filter-name-input")
            .fill_width(),
    )
}

fn tag_filter_row(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::form_row(
        "tags",
        ui::text("Tags").key("filter-tags-label"),
        ui::text_input(state.tag_filter().to_owned())
            .placeholder("Any")
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::TagFilterInput(message))
            })
            .id(TAG_FILTER_INPUT_ID)
            .key("filter-tags-input")
            .fill_width(),
    )
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

    #[test]
    fn filter_section_replaces_type_value_with_tag_text_input() {
        let state = FolderBrowserState::load_default();

        let frame = filter_section(&state)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, 76.0));
        let inputs = frame.paint_plan.text_inputs().collect::<Vec<_>>();

        assert!(
            frame.paint_plan.contains_text("Tags"),
            "tag filter label should be projected"
        );
        assert!(
            !frame.paint_plan.contains_text("Type"),
            "old type filter label should be removed"
        );
        assert_eq!(
            inputs
                .iter()
                .map(|input| input.widget_id)
                .collect::<Vec<_>>(),
            vec![NAME_FILTER_INPUT_ID, TAG_FILTER_INPUT_ID]
        );
    }
}
