use radiant::{prelude as ui, widgets::TextInputMessage};

mod projection;

use self::projection::{
    CurationFilterRowProjection, CurationFilterToggleProjection, HarvestFilterRowProjection,
    HarvestFilterToggleProjection, PlaybackTypeFilterRowProjection,
    PlaybackTypeFilterToggleProjection, RatingFilterRowProjection, RatingFilterToggleProjection,
    TextFilterField, TextFilterRowProjection, filter_rows_projection,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::FilterSectionViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::ui::ids as widget_ids;

pub(super) const FILTER_ROW_HEIGHT: f32 = 24.0;
pub(super) const FILTER_ROW_SPACING: f32 = 1.0;
const FILTER_CLEAR_BUTTON_SIZE: f32 = 20.0;
const FILTER_LABEL_WIDTH: f32 = 54.0;
const FILTER_CONTROL_SPACING: f32 = 2.0;
const FILTER_LABEL_CONTROL_SPACING: f32 = 6.0;
const HARVEST_FILTER_CONTROL_HEIGHT: f32 = FILTER_CLEAR_BUTTON_SIZE * 2.0 + FILTER_CONTROL_SPACING;
pub(super) const FILTER_CONTROLS_CONTENT_HEIGHT: f32 =
    FILTER_ROW_HEIGHT * 5.0 + HARVEST_FILTER_CONTROL_HEIGHT + FILTER_ROW_SPACING * 5.0;
const CURATION_FILTER_TOGGLE_WIDTH: f32 = 46.0;
const HARVEST_FAMILY_TOGGLE_WIDTH: f32 = 22.0;
const HARVEST_FILTER_TOGGLE_WIDTH: f32 = 30.0;
const HARVEST_FILTER_TOP_ROW_TOGGLE_COUNT: usize = 4;
const PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH: f32 = 64.0;
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
/// Scope for automation-facing curation filter toggle ids.
const AUTOMATION_CURATION_FILTER_TOGGLE_SCOPE: u64 =
    widget_ids::AUTOMATION_CURATION_FILTER_TOGGLE_SCOPE;
/// Scope for automation-facing harvest filter toggle ids.
const AUTOMATION_HARVEST_FILTER_TOGGLE_SCOPE: u64 =
    widget_ids::AUTOMATION_HARVEST_FILTER_TOGGLE_SCOPE;
/// Scope for automation-facing rating filter toggle ids.
const AUTOMATION_RATING_FILTER_TOGGLE_SCOPE: u64 =
    widget_ids::AUTOMATION_RATING_FILTER_TOGGLE_SCOPE;

pub(super) fn filter_rows(model: &FilterSectionViewModel) -> [ui::View<GuiMessage>; 6] {
    let projection = filter_rows_projection(model);
    [
        text_filter_row(projection.name_filter),
        text_filter_row(projection.tag_filter),
        curation_filter_row(projection.curation),
        harvest_filter_row(projection.harvest),
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
        .spacing(FILTER_CONTROL_SPACING)
        .fill_width()
        .height(FILTER_CLEAR_BUTTON_SIZE),
        "filter-type-row",
    )
}

fn curation_filter_row(row: CurationFilterRowProjection) -> ui::View<GuiMessage> {
    filter_labeled_control_row(
        filter_row_label(row.label),
        ui::row(
            row.toggles
                .iter()
                .map(curation_filter_toggle)
                .collect::<Vec<_>>(),
        )
        .spacing(FILTER_CONTROL_SPACING)
        .fill_width()
        .height(FILTER_CLEAR_BUTTON_SIZE),
        "filter-curation-row",
    )
}

fn curation_filter_toggle(toggle: &CurationFilterToggleProjection) -> ui::View<GuiMessage> {
    let scope = toggle.scope;
    ui::selectable(toggle.label, toggle.active)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::SetCurationScope(scope, enabled))
        })
        .id(automation_curation_filter_toggle_id(toggle.label))
        .size(CURATION_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE)
}

/// Automation-facing id for a curation filter toggle.
pub(super) fn automation_curation_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_CURATION_FILTER_TOGGLE_SCOPE, label)
}

fn harvest_filter_row(row: HarvestFilterRowProjection) -> ui::View<GuiMessage> {
    let help_tooltips_enabled = row.help_tooltips_enabled;
    let mut top_controls = Vec::with_capacity(HARVEST_FILTER_TOP_ROW_TOGGLE_COUNT + 1);
    let mut bottom_controls = Vec::with_capacity(
        row.toggles
            .len()
            .saturating_sub(HARVEST_FILTER_TOP_ROW_TOGGLE_COUNT),
    );

    top_controls.push(harvest_family_toggle(
        row.family_available,
        row.family_open,
        help_tooltips_enabled,
    ));

    for (index, toggle) in row.toggles.iter().enumerate() {
        let control = harvest_filter_toggle(toggle, help_tooltips_enabled);
        if index < HARVEST_FILTER_TOP_ROW_TOGGLE_COUNT {
            top_controls.push(control);
        } else {
            bottom_controls.push(control);
        }
    }

    let controls = ui::column([
        ui::row(top_controls)
            .spacing(FILTER_CONTROL_SPACING)
            .fill_width()
            .height(FILTER_CLEAR_BUTTON_SIZE),
        ui::row(bottom_controls)
            .spacing(FILTER_CONTROL_SPACING)
            .fill_width()
            .height(FILTER_CLEAR_BUTTON_SIZE),
    ])
    .spacing(FILTER_CONTROL_SPACING)
    .fill_width()
    .height(HARVEST_FILTER_CONTROL_HEIGHT);

    filter_labeled_control_row_with_height(
        filter_row_label(row.label),
        controls,
        "filter-harvest-row",
        HARVEST_FILTER_CONTROL_HEIGHT,
        HARVEST_FILTER_CONTROL_HEIGHT,
    )
}

fn harvest_family_toggle(
    available: bool,
    open: bool,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    ui::disclosure_button(open)
        .enabled(available)
        .active(open)
        .message(GuiMessage::ToggleHarvestFamilyPanel)
        .id(widget_ids::HARVEST_FAMILY_TOGGLE_ID)
        .size(HARVEST_FAMILY_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE)
        .tooltip_if(
            help_tooltips_enabled,
            if available {
                "Show harvest family details for the selected sample."
            } else {
                "Select a harvest-tracked sample to show harvest family details."
            },
        )
}

fn harvest_filter_toggle(
    toggle: &HarvestFilterToggleProjection,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    let filter = toggle.filter;
    ui::selectable(toggle.label, toggle.active)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::SetHarvestFilter(filter, enabled))
        })
        .id(automation_harvest_filter_toggle_id(toggle.label))
        .size(HARVEST_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE)
        .tooltip_if(help_tooltips_enabled, toggle.tooltip)
}

/// Automation-facing id for a harvest filter toggle.
pub(super) fn automation_harvest_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_HARVEST_FILTER_TOGGLE_SCOPE, label)
}

fn playback_type_filter_toggle(
    toggle: &PlaybackTypeFilterToggleProjection,
) -> ui::View<GuiMessage> {
    let filter = toggle.filter;
    ui::selectable(toggle.label, toggle.active)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::TogglePlaybackTypeFilter(
                filter, enabled,
            ))
        })
        .id(automation_playback_type_filter_toggle_id(toggle.label))
        .size(PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH, FILTER_CLEAR_BUTTON_SIZE)
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
    filter_labeled_control_row_with_height(
        label,
        control,
        key,
        FILTER_ROW_HEIGHT,
        FILTER_CLEAR_BUTTON_SIZE,
    )
}

fn filter_labeled_control_row_with_height(
    label: ui::View<GuiMessage>,
    control: ui::View<GuiMessage>,
    key: &'static str,
    row_height: f32,
    cell_height: f32,
) -> ui::View<GuiMessage> {
    ui::row([
        centered_filter_label_cell(label, row_height),
        control.fill_width().height(cell_height),
    ])
    .key(format!("filter-row-{key}"))
    .fill_width()
    .height(row_height)
    .spacing(FILTER_LABEL_CONTROL_SPACING)
}

fn centered_filter_label_cell(
    label: ui::View<GuiMessage>,
    row_height: f32,
) -> ui::View<GuiMessage> {
    let padding = ((row_height - FILTER_CLEAR_BUTTON_SIZE) * 0.5).max(0.0);
    ui::column([
        ui::spacer().fill_width().height(padding),
        label
            .width(FILTER_LABEL_WIDTH)
            .height(FILTER_CLEAR_BUTTON_SIZE),
        ui::spacer().fill_width().height(padding),
    ])
    .width(FILTER_LABEL_WIDTH)
    .height(row_height)
}

pub(super) fn empty_filter_message() -> TextInputMessage {
    TextInputMessage::Changed {
        value: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::app_chrome::view_models::library_sidebar::{
        CurationFilterToggleViewModel, CurationFilterViewModel, FilterSectionViewModel,
        HarvestFilterToggleViewModel, HarvestFilterViewModel, PlaybackTypeFilterToggleViewModel,
        RatingFilterToggleViewModel,
    };
    use crate::native_app::sample_library::folder_browser::model::{
        BrowserCurationScope, HarvestFilter, PlaybackTypeFilter,
    };
    use radiant::prelude::IntoView;

    #[test]
    fn harvest_filter_toggles_expose_full_help_tooltips() {
        let surface = ui::column(filter_rows(&filter_model(true))).into_surface();
        let tooltip = surface
            .find_widget(automation_harvest_filter_toggle_id("Need"))
            .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

        assert_eq!(
            tooltip,
            Some("Files not done or ignored that do not have derivatives yet.")
        );
    }

    fn filter_model(help_tooltips_enabled: bool) -> FilterSectionViewModel {
        FilterSectionViewModel {
            name_filter: String::new(),
            tag_filter: String::new(),
            curation: CurationFilterViewModel {
                toggles: vec![CurationFilterToggleViewModel {
                    scope: BrowserCurationScope::All,
                    label: "All",
                    active: false,
                }],
            },
            harvest: HarvestFilterViewModel {
                toggles: vec![HarvestFilterToggleViewModel {
                    filter: HarvestFilter::NeedsReview,
                    label: "Need",
                    active: false,
                }],
                family_available: false,
                family_open: false,
                help_tooltips_enabled,
            },
            playback_type_filters: vec![PlaybackTypeFilterToggleViewModel {
                filter: PlaybackTypeFilter::OneShot,
                label: "1-Shot",
                active: false,
            }],
            rating_filters: vec![RatingFilterToggleViewModel {
                level: 0,
                label: "U",
                active: false,
            }],
            panel_height: 120.0,
        }
    }
}
