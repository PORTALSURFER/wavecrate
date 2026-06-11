use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::FilterSectionViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::ui::ids as widget_ids;

const FILTER_PANEL_PADDING: f32 = 6.0;
const FILTER_PANEL_HEADER_HEIGHT: f32 = 20.0;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = 4.0;
const NAME_FILTER_INPUT_ID: u64 = widget_ids::NAME_FILTER_INPUT_ID;
const TAG_FILTER_INPUT_ID: u64 = widget_ids::TAG_FILTER_INPUT_ID;

#[cfg(test)]
const FILTER_SECTION_NODE_ID: u64 = widget_ids::FILTER_SECTION_NODE_ID;

pub(super) fn filter_section(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let panel = ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Filter",
            ui::column([name_filter_row(model), tag_filter_row(model)])
                .fill_width()
                .spacing(1.0),
        )
        .trailing_resize_handle("filter-resize-handle", |message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFilterPanel(message))
        })
        .padding(FILTER_PANEL_PADDING)
        .spacing(FILTER_PANEL_HEADER_CONTENT_SPACING)
        .title_height(FILTER_PANEL_HEADER_HEIGHT)
        .height(model.panel_height),
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

fn name_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    ui::form_row(
        "name",
        ui::text("Name").key("filter-name-label"),
        ui::text_input(model.name_filter.clone())
            .placeholder("Any")
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::NameFilterInput(message))
            })
            .id(NAME_FILTER_INPUT_ID)
            .key("filter-name-input")
            .fill_width(),
    )
}

fn tag_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    ui::form_row(
        "tags",
        ui::text("Tags").key("filter-tags-label"),
        ui::text_input(model.tag_filter.clone())
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
    use crate::native_app::sample_library::folder_browser::FolderBrowserState;
    use radiant::prelude::IntoView;

    #[test]
    fn filter_section_layout_uses_configured_height() {
        let mut state = FolderBrowserState::load_default();
        state.resize_filter_panel(ui::DragHandleMessage::started(ui::Point::new(0.0, 200.0)));
        state.resize_filter_panel(ui::DragHandleMessage::moved(ui::Point::new(0.0, 120.0)));
        let model = FilterSectionViewModel::from_folder_browser(&state);

        let layout = ui::column([
            filter_section(&model),
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
        let model = FilterSectionViewModel::from_folder_browser(&state);

        let frame = filter_section(&model)
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
        let model = FilterSectionViewModel::from_folder_browser(&state);

        let frame = filter_section(&model)
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
