use radiant::{prelude as ui, widgets::TextInputMessage};

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::{
    FilterSectionViewModel, PlaybackTypeFilterToggleViewModel, RatingFilterToggleViewModel,
};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::PlaybackTypeFilter;
use crate::native_app::sample_library::folder_browser::view_contract::{
    SIDEBAR_PANEL_HEADER_CONTENT_SPACING, SIDEBAR_PANEL_HEADER_HEIGHT,
};
use crate::native_app::ui::ids as widget_ids;

const FILTER_PANEL_PADDING: f32 = 6.0;
#[cfg(test)]
const FILTER_PANEL_HEADER_HEIGHT: f32 = SIDEBAR_PANEL_HEADER_HEIGHT;
const FILTER_PANEL_HEADER_CONTENT_SPACING: f32 = SIDEBAR_PANEL_HEADER_CONTENT_SPACING;
const FILTER_ROW_HEIGHT: f32 = 24.0;
const FILTER_CLEAR_BUTTON_SIZE: f32 = 20.0;
const FILTER_LABEL_WIDTH: f32 = 38.0;
const FILTER_LABEL_CONTROL_SPACING: f32 = 6.0;
const FILTER_ROW_SPACING: f32 = 1.0;
const PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH: f32 = 58.0;
const RATING_FILTER_TOGGLE_WIDTH: f32 = 20.0;
const RATING_FILTER_SWATCH_SIZE: u8 = 12;
const RATING_FILTER_TRASH_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 238,
    g: 77,
    b: 67,
    a: 235,
};
const RATING_FILTER_UNRATED_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 154,
    g: 158,
    b: 164,
    a: 220,
};
const RATING_FILTER_KEEP_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 122,
    g: 226,
    b: 96,
    a: 235,
};
const NAME_FILTER_INPUT_ID: u64 = widget_ids::NAME_FILTER_INPUT_ID;
const TAG_FILTER_INPUT_ID: u64 = widget_ids::TAG_FILTER_INPUT_ID;
const PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE: u64 = widget_ids::PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE;
const RATING_FILTER_TOGGLE_SCOPE: u64 = widget_ids::RATING_FILTER_TOGGLE_SCOPE;
const FILTER_SECTION_SCROLL_NODE_ID: u64 = widget_ids::FILTER_SECTION_SCROLL_NODE_ID;
const FILTER_RESIZE_HEADER_ID: u64 = widget_ids::FILTER_RESIZE_HEADER_ID;

#[cfg(test)]
const FILTER_SECTION_NODE_ID: u64 = widget_ids::FILTER_SECTION_NODE_ID;

#[cfg(test)]
fn name_filter_clear_button_id() -> u64 {
    ui::text_input_clear_button_id(NAME_FILTER_INPUT_ID)
}

#[cfg(test)]
fn tag_filter_clear_button_id() -> u64 {
    ui::text_input_clear_button_id(TAG_FILTER_INPUT_ID)
}

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
    let rows = [
        name_filter_row(model),
        tag_filter_row(model),
        playback_type_filter_row(model),
        rating_filter_row(model),
    ];
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
        filter_text_input(
            model.name_filter.clone(),
            NAME_FILTER_INPUT_ID,
            FolderBrowserMessage::NameFilterInput,
        ),
        "filter-name-row",
    )
}

fn tag_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    filter_input_row(
        "Tags",
        "filter-tags-label",
        filter_text_input(
            model.tag_filter.clone(),
            TAG_FILTER_INPUT_ID,
            FolderBrowserMessage::TagFilterInput,
        ),
        "filter-tags-row",
    )
}

fn filter_text_input(
    value: String,
    input_id: u64,
    map_message: fn(TextInputMessage) -> FolderBrowserMessage,
) -> ui::View<GuiMessage> {
    ui::text_input(value)
        .placeholder("Any")
        .clear_button(GuiMessage::FolderBrowser(map_message(
            empty_filter_message(),
        )))
        .id(input_id)
        .message_event(move |message| GuiMessage::FolderBrowser(map_message(message)))
}

fn playback_type_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let label = ui::text_line("Type", FILTER_CLEAR_BUTTON_SIZE).key("filter-type-label");
    filter_labeled_control_row(
        label,
        ui::row(
            model
                .playback_type_filters
                .iter()
                .map(playback_type_filter_toggle)
                .collect::<Vec<_>>(),
        )
        .spacing(4.0)
        .fill_width()
        .height(FILTER_CLEAR_BUTTON_SIZE),
        "filter-type-row",
    )
}

fn playback_type_filter_toggle(toggle: &PlaybackTypeFilterToggleViewModel) -> ui::View<GuiMessage> {
    let filter = toggle.filter;
    ui::selectable(toggle.label, toggle.active)
        .style(ui::WidgetStyle::subtle(playback_type_filter_tone(filter)))
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::TogglePlaybackTypeFilter(
                filter, enabled,
            ))
        })
        .id(playback_type_filter_toggle_id(toggle.label))
        .size(PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE)
}

fn playback_type_filter_tone(filter: PlaybackTypeFilter) -> ui::WidgetTone {
    match filter {
        PlaybackTypeFilter::OneShot => ui::WidgetTone::Neutral,
        PlaybackTypeFilter::Loop => ui::WidgetTone::Accent,
    }
}

fn playback_type_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE, label)
}

fn rating_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let label = ui::text_line("Rating", FILTER_CLEAR_BUTTON_SIZE).key("filter-rating-label");
    filter_labeled_control_row(
        label,
        ui::row(
            model
                .rating_filters
                .iter()
                .map(rating_filter_toggle)
                .collect::<Vec<_>>(),
        )
        .spacing(1.0)
        .fill_width()
        .height(FILTER_CLEAR_BUTTON_SIZE),
        "filter-rating-row",
    )
}

fn rating_filter_toggle(toggle: &RatingFilterToggleViewModel) -> ui::View<GuiMessage> {
    let level = toggle.level;
    let input = ui::selectable("", toggle.active)
        .style(ui::WidgetStyle::subtle(rating_filter_tone(level)))
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ToggleRatingFilter(level, enabled))
        })
        .id(rating_filter_toggle_id(toggle.label))
        .size(RATING_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE);
    let swatch = ui::color_marker(Some(rating_filter_swatch_color(level, toggle.active)))
        .side(RATING_FILTER_SWATCH_SIZE)
        .inset(0)
        .align(ui::ColorMarkerAlign::Center)
        .view()
        .size(RATING_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE);

    ui::input_underlay(swatch, input)
        .key(format!("filter-rating-swatch-toggle-{}", toggle.label))
        .size(RATING_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE)
}

fn rating_filter_tone(level: i8) -> ui::WidgetTone {
    if level < 0 {
        ui::WidgetTone::Danger
    } else if level > 0 {
        ui::WidgetTone::Accent
    } else {
        ui::WidgetTone::Neutral
    }
}

fn rating_filter_swatch_color(level: i8, active: bool) -> ui::Rgba8 {
    let color = if level < 0 {
        RATING_FILTER_TRASH_COLOR
    } else if level > 0 {
        RATING_FILTER_KEEP_COLOR
    } else {
        RATING_FILTER_UNRATED_COLOR
    };
    if active {
        color
    } else {
        color.with_alpha(color.a.saturating_sub(68))
    }
}

fn rating_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(RATING_FILTER_TOGGLE_SCOPE, label)
}

fn filter_input_row(
    label: &'static str,
    label_key: &'static str,
    control: ui::View<GuiMessage>,
    key: &'static str,
) -> ui::View<GuiMessage> {
    let label = ui::text_line(label, FILTER_CLEAR_BUTTON_SIZE).key(label_key);
    filter_labeled_control_row(label, control, key)
}

fn filter_labeled_control_row(
    label: ui::View<GuiMessage>,
    control: ui::View<GuiMessage>,
    key: &'static str,
) -> ui::View<GuiMessage> {
    ui::form_row_from_parts(
        ui::FormRowParts::new(key, label, control)
            .height(FILTER_ROW_HEIGHT)
            .label_width(FILTER_LABEL_WIDTH)
            .cell_height(FILTER_CLEAR_BUTTON_SIZE)
            .padding_x(0.0)
            .padding_y(0.0)
            .spacing(FILTER_LABEL_CONTROL_SPACING)
            .hoverable(false),
    )
    .key(key)
    .fill_width()
    .height(FILTER_ROW_HEIGHT)
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
    use radiant::widgets::{ButtonMessage, SelectableMessage};

    const FILTER_SECTION_TEST_FRAME_HEIGHT: f32 = 140.0;

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
                ui::WidgetOutput::typed(drag),
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

        let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
            240.0,
            FILTER_SECTION_TEST_FRAME_HEIGHT,
        ));
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

        let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
            240.0,
            FILTER_SECTION_TEST_FRAME_HEIGHT,
        ));
        let inputs = frame.paint_plan.text_inputs().collect::<Vec<_>>();

        assert!(frame.paint_plan.contains_text("Name"));
        assert!(frame.paint_plan.contains_text("Tags"));
        assert!(frame.paint_plan.contains_text("Type"));
        assert!(frame.paint_plan.contains_text("Rating"));
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

        let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
            240.0,
            FILTER_SECTION_TEST_FRAME_HEIGHT,
        ));

        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(name_filter_clear_button_id()),
            None
        );
        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(tag_filter_clear_button_id()),
            None
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                name_filter_clear_button_id(),
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

        let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
            240.0,
            FILTER_SECTION_TEST_FRAME_HEIGHT,
        ));

        assert!(
            frame
                .paint_plan
                .first_widget_rect(name_filter_clear_button_id())
                .is_some()
        );
        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(tag_filter_clear_button_id()),
            None
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                name_filter_clear_button_id(),
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

        let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
            240.0,
            FILTER_SECTION_TEST_FRAME_HEIGHT,
        ));

        assert_eq!(
            frame
                .paint_plan
                .first_widget_rect(name_filter_clear_button_id()),
            None
        );
        assert!(
            frame
                .paint_plan
                .first_widget_rect(tag_filter_clear_button_id())
                .is_some()
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                tag_filter_clear_button_id(),
                ui::WidgetOutput::typed(ButtonMessage::Activate),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::TagFilterInput(empty_filter_message())
            ))
        );
    }

    #[test]
    fn filter_section_projects_playback_type_toggles_and_dispatches_changes() {
        let mut state = FolderBrowserState::load_default();
        state.set_playback_type_filter(PlaybackTypeFilter::Loop, true);
        let model = FilterSectionViewModel::from_folder_browser(&state);
        let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
            240.0,
            FILTER_SECTION_TEST_FRAME_HEIGHT,
        ));

        assert!(
            frame
                .paint_plan
                .first_widget_rect(playback_type_filter_toggle_id("1-Shot"))
                .is_some()
        );
        assert!(
            frame
                .paint_plan
                .first_widget_rect(playback_type_filter_toggle_id("Loop"))
                .is_some()
        );
        assert!(frame.paint_plan.contains_text("1-Shot"));
        assert!(frame.paint_plan.contains_text("Loop"));
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                playback_type_filter_toggle_id("1-Shot"),
                ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: true }),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::TogglePlaybackTypeFilter(PlaybackTypeFilter::OneShot, true)
            ))
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                playback_type_filter_toggle_id("Loop"),
                ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::TogglePlaybackTypeFilter(PlaybackTypeFilter::Loop, false)
            ))
        );
    }

    #[test]
    fn filter_section_projects_rating_toggles_and_dispatches_changes() {
        let mut state = FolderBrowserState::load_default();
        state.set_rating_filter(-3, true);
        state.set_rating_filter(0, true);
        let model = FilterSectionViewModel::from_folder_browser(&state);
        let frame = filter_section(&model).view_frame_at_size_with_default_theme(ui::Vector2::new(
            240.0,
            FILTER_SECTION_TEST_FRAME_HEIGHT,
        ));

        assert!(
            frame
                .paint_plan
                .first_widget_rect(rating_filter_toggle_id("T3"))
                .is_some()
        );
        assert!(
            frame
                .paint_plan
                .first_widget_rect(rating_filter_toggle_id("U"))
                .is_some()
        );
        assert!(
            frame
                .paint_plan
                .first_widget_rect(rating_filter_toggle_id("K4"))
                .is_some()
        );
        assert!(frame.paint_plan.fill_rects().any(|fill| {
            fill.color == rating_filter_swatch_color(-3, true)
                && fill.rect.width() == RATING_FILTER_SWATCH_SIZE as f32
        }));
        assert!(
            frame
                .paint_plan
                .fill_rects()
                .any(|fill| fill.color == rating_filter_swatch_color(1, false))
        );
        assert!(
            !frame
                .paint_plan
                .text_labels()
                .any(|label| matches!(label, "T3" | "T2" | "T1" | "U" | "K1" | "K2" | "K3" | "K4"))
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                rating_filter_toggle_id("K4"),
                ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: true }),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::ToggleRatingFilter(4, true)
            ))
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                rating_filter_toggle_id("U"),
                ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::ToggleRatingFilter(0, false)
            ))
        );
        assert_eq!(
            filter_section(&model).view_dispatch_widget_output(
                rating_filter_toggle_id("T3"),
                ui::WidgetOutput::typed(SelectableMessage::SelectionChanged { selected: false }),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::ToggleRatingFilter(-3, false)
            ))
        );
    }
}
