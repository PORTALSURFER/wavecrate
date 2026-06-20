use radiant::{prelude as ui, widgets::TextInputMessage};

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::panel_chrome::sidebar_resize_header;
use crate::native_app::app_chrome::view_models::library_sidebar::FilterSectionViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
#[cfg(test)]
use crate::native_app::sample_library::folder_browser::view_contract::SIDEBAR_PANEL_HEADER_HEIGHT;
use crate::native_app::ui::ids as widget_ids;

const FILTER_PANEL_PADDING: f32 = 6.0;
#[cfg(test)]
const FILTER_PANEL_HEADER_HEIGHT: f32 = SIDEBAR_PANEL_HEADER_HEIGHT;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
const FILTER_ROW_HEIGHT: f32 = 24.0;
const FILTER_CLEAR_BUTTON_SIZE: f32 = 20.0;
const FILTER_LABEL_WIDTH: f32 = 38.0;
const FILTER_LABEL_CONTROL_SPACING: f32 = 6.0;
const FILTER_INPUT_CLEAR_SPACING: f32 = 4.0;
const FILTER_ROW_SPACING: f32 = 1.0;
const NAME_FILTER_INPUT_ID: u64 = widget_ids::NAME_FILTER_INPUT_ID;
const TAG_FILTER_INPUT_ID: u64 = widget_ids::TAG_FILTER_INPUT_ID;
const FILTER_SECTION_SCROLL_NODE_ID: u64 = widget_ids::FILTER_SECTION_SCROLL_NODE_ID;
const NAME_FILTER_CLEAR_BUTTON_ID: u64 = widget_ids::NAME_FILTER_CLEAR_BUTTON_ID;
const TAG_FILTER_CLEAR_BUTTON_ID: u64 = widget_ids::TAG_FILTER_CLEAR_BUTTON_ID;
const FILTER_RESIZE_HEADER_ID: u64 = widget_ids::FILTER_RESIZE_HEADER_ID;

#[cfg(test)]
const FILTER_SECTION_NODE_ID: u64 = widget_ids::FILTER_SECTION_NODE_ID;

pub(super) fn filter_section(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let panel = ui::column([filter_resize_header(), filter_controls(model)])
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
        .padding(FILTER_PANEL_PADDING)
        .spacing(FILTER_PANEL_HEADER_CONTENT_SPACING)
        .height(model.panel_height)
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

fn filter_resize_header() -> ui::View<GuiMessage> {
    sidebar_resize_header("filter-resize-header", FILTER_RESIZE_HEADER_ID, |message| {
        GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeFilterPanel(message))
    })
}

fn filter_controls(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let rows = [name_filter_row(model), tag_filter_row(model)];
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

fn name_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    filter_input_row(
        "Name",
        "filter-name-label",
        ui::text_input(model.name_filter.clone())
            .placeholder("Any")
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::NameFilterInput(message))
            })
            .id(NAME_FILTER_INPUT_ID)
            .key("filter-name-input")
            .fill_width(),
        filter_clear_slot(
            !model.name_filter.is_empty(),
            NAME_FILTER_CLEAR_BUTTON_ID,
            "filter-name-clear-button",
            GuiMessage::FolderBrowser(
                FolderBrowserMessage::NameFilterInput(empty_filter_message()),
            ),
        ),
        "filter-name-row",
    )
}

fn tag_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    filter_input_row(
        "Tags",
        "filter-tags-label",
        ui::text_input(model.tag_filter.clone())
            .placeholder("Any")
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::TagFilterInput(message))
            })
            .id(TAG_FILTER_INPUT_ID)
            .key("filter-tags-input")
            .fill_width(),
        filter_clear_slot(
            !model.tag_filter.is_empty(),
            TAG_FILTER_CLEAR_BUTTON_ID,
            "filter-tags-clear-button",
            GuiMessage::FolderBrowser(FolderBrowserMessage::TagFilterInput(empty_filter_message())),
        ),
        "filter-tags-row",
    )
}

fn filter_input_row(
    label: &'static str,
    label_key: &'static str,
    input: ui::View<GuiMessage>,
    clear_slot: ui::View<GuiMessage>,
    key: &'static str,
) -> ui::View<GuiMessage> {
    let label = ui::text_line(label, FILTER_CLEAR_BUTTON_SIZE)
        .key(label_key)
        .width(FILTER_LABEL_WIDTH);
    let control = ui::row([input, clear_slot])
        .spacing(FILTER_INPUT_CLEAR_SPACING)
        .fill_width()
        .height(FILTER_CLEAR_BUTTON_SIZE);

    ui::row([label, control])
        .key(key)
        .spacing(FILTER_LABEL_CONTROL_SPACING)
        .fill_width()
        .height(FILTER_ROW_HEIGHT)
}

fn filter_clear_slot(
    active: bool,
    widget_id: u64,
    key: &'static str,
    message: GuiMessage,
) -> ui::View<GuiMessage> {
    if active {
        ui::close_button()
            .subtle()
            .message(message)
            .id(widget_id)
            .key(key)
            .size(FILTER_CLEAR_BUTTON_SIZE, FILTER_CLEAR_BUTTON_SIZE)
    } else {
        ui::spacer()
            .width(FILTER_CLEAR_BUTTON_SIZE)
            .height(FILTER_CLEAR_BUTTON_SIZE)
    }
}

fn empty_filter_message() -> TextInputMessage {
    TextInputMessage::Changed {
        value: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::sample_library::folder_browser::FolderBrowserState;
    use radiant::prelude::IntoView;
    use radiant::widgets::ButtonMessage;

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
    fn filter_resize_header_uses_full_width_hit_target() {
        let state = FolderBrowserState::load_default();
        let model = FilterSectionViewModel::from_folder_browser(&state);
        let layout = filter_section(&model).view_layout_at_size(ui::Vector2::new(240.0, 120.0));
        let section = layout
            .rects
            .get(&FILTER_SECTION_NODE_ID)
            .expect("filter section layout rect");
        let header = layout
            .rects
            .get(&FILTER_RESIZE_HEADER_ID)
            .expect("filter resize header layout rect");
        let drag =
            ui::DragHandleMessage::started(ui::Point::new(header.center().x, header.center().y));

        assert!(
            header.width() >= section.width() - FILTER_PANEL_PADDING * 2.0,
            "filter resize header should span the useful panel width, section={section:?}, header={header:?}"
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                FILTER_RESIZE_HEADER_ID,
                ui::WidgetOutput::typed(drag.clone()),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::ResizeFilterPanel(drag)
            ))
        );
    }

    #[test]
    fn filter_section_controls_scroll_when_panel_is_cramped() {
        let state = FolderBrowserState::load_default();
        let model = FilterSectionViewModel {
            panel_height: FILTER_PANEL_HEADER_HEIGHT + FILTER_PANEL_PADDING * 2.0 + 18.0,
            ..FilterSectionViewModel::from_folder_browser(&state)
        };

        let layout = ui::column([
            filter_section(&model),
            ui::spacer().fill_width().fill_height(),
        ])
        .view_layout_at_size(ui::Vector2::new(240.0, 600.0));
        let overflow = layout
            .overflow_flags
            .get(&FILTER_SECTION_SCROLL_NODE_ID)
            .expect("filter controls should have a scroll viewport");

        assert!(
            overflow.y,
            "cramped filter controls should scroll vertically"
        );
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
    fn filter_section_projects_tag_text_input_with_row_labels() {
        let state = FolderBrowserState::load_default();
        let model = FilterSectionViewModel::from_folder_browser(&state);

        let frame = filter_section(&model)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, 76.0));
        let inputs = frame.paint_plan.text_inputs().collect::<Vec<_>>();

        assert!(frame.paint_plan.contains_text("Name"));
        assert!(frame.paint_plan.contains_text("Tags"));
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

    #[test]
    fn filter_section_hides_clear_buttons_when_filters_are_empty() {
        let state = FolderBrowserState::load_default();
        let model = FilterSectionViewModel::from_folder_browser(&state);

        let frame = filter_section(&model)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, 76.0));

        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(NAME_FILTER_CLEAR_BUTTON_ID),
            None
        );
        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(TAG_FILTER_CLEAR_BUTTON_ID),
            None
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                NAME_FILTER_CLEAR_BUTTON_ID,
                ui::WidgetOutput::typed(ButtonMessage::Activate),
            ),
            None
        );
    }

    #[test]
    fn filter_section_projects_name_clear_button_for_active_name_filter() {
        let state = FolderBrowserState::load_default();
        let model = FilterSectionViewModel {
            name_filter: String::from("kick"),
            ..FilterSectionViewModel::from_folder_browser(&state)
        };

        let frame = filter_section(&model)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, 76.0));

        assert!(
            frame
                .paint_plan
                .first_widget_rect(NAME_FILTER_CLEAR_BUTTON_ID)
                .is_some()
        );
        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(TAG_FILTER_CLEAR_BUTTON_ID),
            None
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                NAME_FILTER_CLEAR_BUTTON_ID,
                ui::WidgetOutput::typed(ButtonMessage::Activate),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::NameFilterInput(empty_filter_message())
            ))
        );
    }

    #[test]
    fn filter_section_projects_tag_clear_button_for_active_tag_filter() {
        let state = FolderBrowserState::load_default();
        let model = FilterSectionViewModel {
            tag_filter: String::from("drum"),
            ..FilterSectionViewModel::from_folder_browser(&state)
        };

        let frame = filter_section(&model)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(240.0, 76.0));

        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(NAME_FILTER_CLEAR_BUTTON_ID),
            None
        );
        assert!(
            frame
                .paint_plan
                .first_widget_rect(TAG_FILTER_CLEAR_BUTTON_ID)
                .is_some()
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                TAG_FILTER_CLEAR_BUTTON_ID,
                ui::WidgetOutput::typed(ButtonMessage::Activate),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::TagFilterInput(empty_filter_message())
            ))
        );
    }
}
