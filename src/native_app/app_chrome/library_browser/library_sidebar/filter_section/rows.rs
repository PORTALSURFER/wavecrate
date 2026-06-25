use radiant::{prelude as ui, widgets::TextInputMessage};

mod projection;

use self::projection::{
    CurationFilterRowProjection, PlaybackTypeFilterRowProjection,
    PlaybackTypeFilterToggleProjection, RatingFilterRowProjection, RatingFilterToggleProjection,
    TextFilterField, TextFilterRowProjection, filter_rows_projection,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::FilterSectionViewModel;
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
/// Scope for automation-facing playback-type filter toggle ids.
const AUTOMATION_PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE: u64 =
    widget_ids::AUTOMATION_PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE;
/// Scope for automation-facing rating filter toggle ids.
const AUTOMATION_RATING_FILTER_TOGGLE_SCOPE: u64 =
    widget_ids::AUTOMATION_RATING_FILTER_TOGGLE_SCOPE;

pub(super) fn filter_rows(model: &FilterSectionViewModel) -> [ui::View<GuiMessage>; 5] {
    let projection = filter_rows_projection(model);
    [
        text_filter_row(projection.name_filter),
        text_filter_row(projection.tag_filter),
        curation_filter_row(projection.curation),
        playback_type_filter_row(projection.playback_type),
        rating_filter_row(projection.rating),
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

fn text_filter_row(projection: TextFilterRowProjection) -> ui::View<GuiMessage> {
    filter_input_row(
        projection.label,
        filter_text_input(
            projection.value,
            projection.placeholder,
            text_filter_input_id(projection.field),
            text_filter_message_mapper(projection.field),
        ),
        text_filter_row_key(projection.field),
    )
}

fn filter_text_input(
    value: String,
    placeholder: &'static str,
    input_id: u64,
    map_message: fn(TextInputMessage) -> FolderBrowserMessage,
) -> ui::View<GuiMessage> {
    ui::text_input(value)
        .placeholder(placeholder)
        .clear_button(GuiMessage::FolderBrowser(map_message(
            empty_filter_message(),
        )))
        .id(input_id)
        .message_event(move |message| GuiMessage::FolderBrowser(map_message(message)))
}

fn text_filter_input_id(field: TextFilterField) -> u64 {
    match field {
        TextFilterField::Name => NAME_FILTER_INPUT_ID,
        TextFilterField::Tags => TAG_FILTER_INPUT_ID,
    }
}

fn text_filter_message_mapper(
    field: TextFilterField,
) -> fn(TextInputMessage) -> FolderBrowserMessage {
    match field {
        TextFilterField::Name => FolderBrowserMessage::NameFilterInput,
        TextFilterField::Tags => FolderBrowserMessage::TagFilterInput,
    }
}

fn text_filter_row_key(field: TextFilterField) -> &'static str {
    match field {
        TextFilterField::Name => "filter-name-row",
        TextFilterField::Tags => "filter-tags-row",
    }
}

fn playback_type_filter_row(row: PlaybackTypeFilterRowProjection) -> ui::View<GuiMessage> {
    let label = filter_row_label(row.label);
    filter_labeled_control_row(
        label,
        ui::row(
            row.toggles
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

fn curation_filter_row(row: CurationFilterRowProjection) -> ui::View<GuiMessage> {
    let active = row.active;
    filter_labeled_control_row(
        filter_row_label(row.label),
        ui::row([
            ui::selectable("Source", active)
                .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
                .message(|enabled| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ToggleCurationMode(enabled))
                })
                .size(PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE),
            ui::text(row.status)
                .muted_text()
                .height(FILTER_CLEAR_BUTTON_SIZE)
                .width(34.0),
        ])
        .spacing(4.0)
        .fill_width()
        .height(FILTER_CLEAR_BUTTON_SIZE),
        "filter-curation-row",
    )
}

fn playback_type_filter_toggle(
    toggle: &PlaybackTypeFilterToggleProjection,
) -> ui::View<GuiMessage> {
    let filter = toggle.filter;
    ui::selectable(toggle.label, toggle.active)
        .style(ui::WidgetStyle::subtle(playback_type_filter_tone(filter)))
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::TogglePlaybackTypeFilter(
                filter, enabled,
            ))
        })
        .id(automation_playback_type_filter_toggle_id(toggle.label))
        .size(PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE)
}

fn playback_type_filter_tone(filter: PlaybackTypeFilter) -> ui::WidgetTone {
    match filter {
        PlaybackTypeFilter::OneShot => ui::WidgetTone::Neutral,
        PlaybackTypeFilter::Loop => ui::WidgetTone::Accent,
    }
}

/// Automation-facing id for a playback-type filter toggle.
pub(super) fn automation_playback_type_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE, label)
}

fn rating_filter_row(row: RatingFilterRowProjection) -> ui::View<GuiMessage> {
    let label = filter_row_label(row.label);
    filter_labeled_control_row(
        label,
        ui::row(
            row.toggles
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

fn rating_filter_toggle(toggle: &RatingFilterToggleProjection) -> ui::View<GuiMessage> {
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
        .id(automation_rating_filter_toggle_id(toggle.label))
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

/// Automation-facing id for a rating filter toggle.
pub(super) fn automation_rating_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_RATING_FILTER_TOGGLE_SCOPE, label)
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
    ui::dense_form_row(key, label, control, FILTER_LABEL_WIDTH)
        .fill_width()
        .height(FILTER_ROW_HEIGHT)
}

pub(super) fn empty_filter_message() -> TextInputMessage {
    TextInputMessage::Changed {
        value: String::new(),
    }
}
