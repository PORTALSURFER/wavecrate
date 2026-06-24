use radiant::{prelude as ui, widgets::TextInputMessage};

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::{
    FilterSectionViewModel, PlaybackTypeFilterToggleViewModel, RatingFilterToggleViewModel,
};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::PlaybackTypeFilter;
use crate::native_app::ui::ids as widget_ids;

pub(super) const FILTER_ROW_HEIGHT: f32 = 24.0;
pub(super) const FILTER_ROW_SPACING: f32 = 1.0;
const FILTER_CLEAR_BUTTON_SIZE: f32 = 20.0;
const FILTER_LABEL_WIDTH: f32 = 38.0;
const PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH: f32 = 58.0;
const RATING_FILTER_TOGGLE_WIDTH: f32 = 20.0;
pub(super) const RATING_FILTER_SWATCH_SIZE: u8 = 12;
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
pub(super) const NAME_FILTER_INPUT_ID: u64 = widget_ids::NAME_FILTER_INPUT_ID;
pub(super) const TAG_FILTER_INPUT_ID: u64 = widget_ids::TAG_FILTER_INPUT_ID;
const PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE: u64 = widget_ids::PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE;
const RATING_FILTER_TOGGLE_SCOPE: u64 = widget_ids::RATING_FILTER_TOGGLE_SCOPE;

pub(super) fn filter_rows(model: &FilterSectionViewModel) -> [ui::View<GuiMessage>; 4] {
    [
        name_filter_row(model),
        tag_filter_row(model),
        playback_type_filter_row(model),
        rating_filter_row(model),
    ]
}

#[cfg(test)]
pub(super) fn name_filter_clear_button_id() -> u64 {
    ui::text_input_clear_button_id(NAME_FILTER_INPUT_ID)
}

#[cfg(test)]
pub(super) fn tag_filter_clear_button_id() -> u64 {
    ui::text_input_clear_button_id(TAG_FILTER_INPUT_ID)
}

fn name_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    filter_input_row(
        "Name",
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
    let label = filter_row_label("Type");
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

pub(super) fn playback_type_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE, label)
}

fn rating_filter_row(model: &FilterSectionViewModel) -> ui::View<GuiMessage> {
    let label = filter_row_label("Rating");
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
    ui::selectable("", toggle.active)
        .style(ui::WidgetStyle::subtle(rating_filter_tone(level)))
        .color_marker(Some(rating_filter_swatch_color(level, toggle.active)))
        .color_marker_side(RATING_FILTER_SWATCH_SIZE)
        .color_marker_inset(0)
        .color_marker_align(ui::ColorMarkerAlign::Center)
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ToggleRatingFilter(level, enabled))
        })
        .id(rating_filter_toggle_id(toggle.label))
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

pub(super) fn rating_filter_swatch_color(level: i8, active: bool) -> ui::Rgba8 {
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

pub(super) fn rating_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(RATING_FILTER_TOGGLE_SCOPE, label)
}

fn filter_input_row(
    label: &'static str,
    control: ui::View<GuiMessage>,
    key: &'static str,
) -> ui::View<GuiMessage> {
    filter_labeled_control_row(filter_row_label(label), control, key)
}

fn filter_row_label(label: &'static str) -> ui::View<GuiMessage> {
    ui::text_line(label, FILTER_CLEAR_BUTTON_SIZE)
}

fn filter_labeled_control_row(
    label: ui::View<GuiMessage>,
    control: ui::View<GuiMessage>,
    key: &'static str,
) -> ui::View<GuiMessage> {
    ui::form_row_from_parts(
        ui::FormRowParts::dense(key, label, control).label_width(FILTER_LABEL_WIDTH),
    )
    .fill_width()
    .height(FILTER_ROW_HEIGHT)
}

pub(super) fn empty_filter_message() -> TextInputMessage {
    TextInputMessage::Changed {
        value: String::new(),
    }
}
