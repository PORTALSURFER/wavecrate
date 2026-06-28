use radiant::{prelude as ui, widgets::TextInputMessage};

mod projection;

use self::projection::{
    CurationFilterOptionProjection, CurationFilterRowProjection, HarvestFilterOptionProjection,
    HarvestFilterRowProjection, PlaybackTypeFilterRowProjection,
    PlaybackTypeFilterToggleProjection, RatingFilterRowProjection, RatingFilterToggleProjection,
    TextFilterField, TextFilterRowProjection, filter_rows_projection,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::FilterSectionViewModel;
use crate::native_app::sample_library::folder_browser::commands::{
    FilterFamily, FolderBrowserMessage,
};
pub(super) use crate::native_app::sample_library::folder_browser::view_contract::{
    FILTER_CONTROLS_CONTENT_HEIGHT, FILTER_ROW_CONTROL_HEIGHT, FILTER_ROW_HEIGHT,
    FILTER_ROW_SPACING,
};
use crate::native_app::ui::ids as widget_ids;

pub(super) const FILTER_ROW_VERTICAL_INSET: f32 =
    (FILTER_ROW_HEIGHT - FILTER_ROW_CONTROL_HEIGHT) * 0.5;
pub(super) const FILTER_LABEL_WIDTH: f32 = 64.0;
const FILTER_CONTROL_SPACING: f32 = 2.0;
pub(super) const FILTER_LABEL_CONTROL_SPACING: f32 = 6.0;
const HARVEST_FAMILY_TOGGLE_WIDTH: f32 = 22.0;
const PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH: f32 = 64.0;
const RATING_FILTER_TOGGLE_WIDTH: f32 = 18.0;
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
const AUTOMATION_CURATION_FILTER_DROPDOWN_OPTION_SCOPE: u64 =
    widget_ids::AUTOMATION_CURATION_FILTER_DROPDOWN_OPTION_SCOPE;
pub(super) const CURATION_FILTER_DROPDOWN_TRIGGER_ID: u64 =
    widget_ids::CURATION_FILTER_DROPDOWN_TRIGGER_ID;
pub(super) const HARVEST_FILTER_DROPDOWN_TRIGGER_ID: u64 =
    widget_ids::HARVEST_FILTER_DROPDOWN_TRIGGER_ID;
/// Scope for automation-facing harvest dropdown option ids.
const AUTOMATION_HARVEST_FILTER_DROPDOWN_OPTION_SCOPE: u64 =
    widget_ids::AUTOMATION_HARVEST_FILTER_DROPDOWN_OPTION_SCOPE;
/// Scope for automation-facing rating filter toggle ids.
const AUTOMATION_RATING_FILTER_TOGGLE_SCOPE: u64 =
    widget_ids::AUTOMATION_RATING_FILTER_TOGGLE_SCOPE;
/// Scope for automation-facing filter family label toggles.
const AUTOMATION_FILTER_FAMILY_LABEL_TOGGLE_SCOPE: u64 =
    widget_ids::AUTOMATION_FILTER_FAMILY_LABEL_TOGGLE_SCOPE;

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
        filter_row_label(projection.label, projection.family, projection.enabled),
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
    let label = filter_row_label(row.label, row.family, row.enabled);
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
        .height(FILTER_ROW_CONTROL_HEIGHT),
        "filter-type-row",
    )
}

fn curation_filter_row(row: CurationFilterRowProjection) -> ui::View<GuiMessage> {
    filter_labeled_control_row(
        filter_row_label(row.label, row.family, row.enabled),
        ui::dropdown_trigger(row.selected_label, row.dropdown_open)
            .toggle_message(GuiMessage::ToggleCurationFilterDropdown)
            .build()
            .id(CURATION_FILTER_DROPDOWN_TRIGGER_ID)
            .fill_width()
            .height(FILTER_ROW_CONTROL_HEIGHT),
        "filter-curation-row",
    )
}

pub(super) fn curation_filter_dropdown_menu(
    model: &FilterSectionViewModel,
) -> Option<(ui::View<GuiMessage>, ui::Vector2)> {
    let row = filter_rows_projection(model).curation;
    if row.dropdown_open {
        let size = ui::Vector2::new(
            f32::from(row.menu_width).max(1.0),
            ui::dropdown_menu_height(row.options.len()),
        );
        Some((
            curation_filter_dropdown_menu_from_options(&row.options, size.x),
            size,
        ))
    } else {
        None
    }
}

fn curation_filter_dropdown_menu_from_options(
    options: &[CurationFilterOptionProjection],
    menu_width: f32,
) -> ui::View<GuiMessage> {
    let option_count = options.len();
    ui::column(options.iter().map(curation_filter_dropdown_option))
        .key("curation-filter-dropdown-menu")
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Strong,
        })
        .padding(4.0)
        .spacing(3.0)
        .width(menu_width.max(1.0))
        .height(ui::dropdown_menu_height(option_count))
}

fn curation_filter_dropdown_option(
    option: &CurationFilterOptionProjection,
) -> ui::View<GuiMessage> {
    let scope = option.scope;
    ui::button(option.label)
        .message(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SetCurationScope(scope, true),
        ))
        .id(automation_curation_filter_dropdown_option_id(option.label))
        .style(ui::WidgetStyle {
            tone: if option.selected {
                ui::WidgetTone::Accent
            } else {
                ui::WidgetTone::Neutral
            },
            prominence: if option.selected {
                ui::WidgetProminence::Strong
            } else {
                ui::WidgetProminence::Subtle
            },
        })
        .fill_width()
        .height(22.0)
}

/// Automation-facing id for a curation dropdown option.
pub(super) fn automation_curation_filter_dropdown_option_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_CURATION_FILTER_DROPDOWN_OPTION_SCOPE, label)
}

fn harvest_filter_row(row: HarvestFilterRowProjection) -> ui::View<GuiMessage> {
    let help_tooltips_enabled = row.help_tooltips_enabled;
    let controls = ui::row([
        harvest_filter_arrow_toggle(row.dropdown_open, help_tooltips_enabled),
        ui::dropdown_trigger(row.selected_label, row.dropdown_open)
            .toggle_message(GuiMessage::ToggleHarvestFilterDropdown)
            .build()
            .id(HARVEST_FILTER_DROPDOWN_TRIGGER_ID)
            .tooltip_if(help_tooltips_enabled, "Choose the Harvest queue to show.")
            .fill_width()
            .height(FILTER_ROW_CONTROL_HEIGHT),
    ])
    .spacing(FILTER_CONTROL_SPACING)
    .fill_width()
    .height(FILTER_ROW_CONTROL_HEIGHT);

    filter_labeled_control_row(
        filter_row_label(row.label, row.family, row.enabled),
        controls,
        "filter-harvest-row",
    )
}

fn harvest_filter_arrow_toggle(open: bool, help_tooltips_enabled: bool) -> ui::View<GuiMessage> {
    ui::disclosure_button(open)
        .active(open)
        .message(GuiMessage::ToggleHarvestFilterDropdown)
        .id(widget_ids::HARVEST_FAMILY_TOGGLE_ID)
        .size(HARVEST_FAMILY_TOGGLE_WIDTH, FILTER_ROW_CONTROL_HEIGHT)
        .tooltip_if(help_tooltips_enabled, "Choose the Harvest queue to show.")
}

pub(super) fn harvest_filter_dropdown_menu(
    model: &FilterSectionViewModel,
) -> Option<(ui::View<GuiMessage>, ui::Vector2)> {
    let row = filter_rows_projection(model).harvest;
    if row.dropdown_open {
        let size = ui::Vector2::new(
            f32::from(row.menu_width).max(1.0),
            ui::dropdown_menu_height(row.options.len()),
        );
        Some((
            harvest_filter_dropdown_menu_from_options(
                &row.options,
                row.help_tooltips_enabled,
                size.x,
            ),
            size,
        ))
    } else {
        None
    }
}

fn harvest_filter_dropdown_menu_from_options(
    options: &[HarvestFilterOptionProjection],
    help_tooltips_enabled: bool,
    menu_width: f32,
) -> ui::View<GuiMessage> {
    let option_count = options.len();
    ui::column(
        options
            .iter()
            .map(|option| harvest_filter_dropdown_option(option, help_tooltips_enabled)),
    )
    .key("harvest-filter-dropdown-menu")
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Neutral,
        prominence: ui::WidgetProminence::Strong,
    })
    .padding(4.0)
    .spacing(3.0)
    .width(menu_width.max(1.0))
    .height(ui::dropdown_menu_height(option_count))
}

fn harvest_filter_dropdown_option(
    option: &HarvestFilterOptionProjection,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    let filter = option.filter;
    ui::button(option.label)
        .message(GuiMessage::FolderBrowser(
            FolderBrowserMessage::SetHarvestFilter(filter, true),
        ))
        .id(automation_harvest_filter_dropdown_option_id(option.label))
        .style(ui::WidgetStyle {
            tone: if option.selected {
                ui::WidgetTone::Accent
            } else {
                ui::WidgetTone::Neutral
            },
            prominence: if option.selected {
                ui::WidgetProminence::Strong
            } else {
                ui::WidgetProminence::Subtle
            },
        })
        .fill_width()
        .height(22.0)
        .tooltip_if(help_tooltips_enabled, option.tooltip)
}

/// Automation-facing id for a harvest dropdown option.
pub(super) fn automation_harvest_filter_dropdown_option_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_HARVEST_FILTER_DROPDOWN_OPTION_SCOPE, label)
}

fn playback_type_filter_toggle(
    toggle: &PlaybackTypeFilterToggleProjection,
) -> ui::View<GuiMessage> {
    let filter = toggle.filter;
    ui::selectable(toggle.label, toggle.active)
        .style(playback_type_filter_toggle_style())
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::TogglePlaybackTypeFilter(
                filter, enabled,
            ))
        })
        .id(automation_playback_type_filter_toggle_id(toggle.label))
        .size(PLAYBACK_TYPE_FILTER_TOGGLE_WIDTH, FILTER_ROW_CONTROL_HEIGHT)
}

fn playback_type_filter_toggle_style() -> ui::WidgetStyle {
    ui::WidgetStyle::subtle(ui::WidgetTone::Accent)
}

/// Automation-facing id for a playback-type filter toggle.
pub(super) fn automation_playback_type_filter_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE, label)
}

fn rating_filter_row(row: RatingFilterRowProjection) -> ui::View<GuiMessage> {
    let label = filter_row_label(row.label, row.family, row.enabled);
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
        .height(FILTER_ROW_CONTROL_HEIGHT),
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
        .size(RATING_FILTER_TOGGLE_WIDTH, FILTER_ROW_CONTROL_HEIGHT)
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
    label: ui::View<GuiMessage>,
    control: ui::View<GuiMessage>,
    key: &'static str,
) -> ui::View<GuiMessage> {
    filter_labeled_control_row(label, control, key)
}

fn filter_row_label(
    label: &'static str,
    family: FilterFamily,
    enabled: bool,
) -> ui::View<GuiMessage> {
    ui::selectable(label, enabled)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .message(move |enabled| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::SetFilterFamilyEnabled(
                family, enabled,
            ))
        })
        .id(automation_filter_family_label_toggle_id(label))
        .size(FILTER_LABEL_WIDTH, FILTER_ROW_CONTROL_HEIGHT)
}

/// Automation-facing id for a filter family label toggle.
pub(super) fn automation_filter_family_label_toggle_id(label: &str) -> u64 {
    ui::stable_widget_id(AUTOMATION_FILTER_FAMILY_LABEL_TOGGLE_SCOPE, label)
}

fn filter_labeled_control_row(
    label: ui::View<GuiMessage>,
    control: ui::View<GuiMessage>,
    key: &'static str,
) -> ui::View<GuiMessage> {
    ui::row([
        centered_filter_label_cell(label),
        centered_filter_control_cell(control),
    ])
    .key(format!("filter-row-{key}"))
    .fill_width()
    .height(FILTER_ROW_HEIGHT)
    .spacing(FILTER_LABEL_CONTROL_SPACING)
}

fn centered_filter_label_cell(label: ui::View<GuiMessage>) -> ui::View<GuiMessage> {
    ui::column([
        ui::spacer().fill_width().height(FILTER_ROW_VERTICAL_INSET),
        label
            .width(FILTER_LABEL_WIDTH)
            .height(FILTER_ROW_CONTROL_HEIGHT),
        ui::spacer().fill_width().height(FILTER_ROW_VERTICAL_INSET),
    ])
    .width(FILTER_LABEL_WIDTH)
    .height(FILTER_ROW_HEIGHT)
}

fn centered_filter_control_cell(control: ui::View<GuiMessage>) -> ui::View<GuiMessage> {
    ui::column([
        ui::spacer().fill_width().height(FILTER_ROW_VERTICAL_INSET),
        control.fill_width().height(FILTER_ROW_CONTROL_HEIGHT),
        ui::spacer().fill_width().height(FILTER_ROW_VERTICAL_INSET),
    ])
    .fill_width()
    .height(FILTER_ROW_HEIGHT)
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
        CurationFilterOptionViewModel, CurationFilterViewModel, FilterSectionViewModel,
        HarvestFilterOptionViewModel, HarvestFilterViewModel, PlaybackTypeFilterToggleViewModel,
        RatingFilterToggleViewModel,
    };
    use crate::native_app::sample_library::folder_browser::model::{
        BrowserCurationScope, HarvestFilter, PlaybackTypeFilter,
    };
    use radiant::prelude::IntoView;

    #[test]
    fn harvest_filter_dropdown_options_expose_full_help_tooltips() {
        let surface = ui::column(filter_rows(&filter_model(true))).into_surface();
        let tooltip = surface
            .find_widget(HARVEST_FILTER_DROPDOWN_TRIGGER_ID)
            .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

        assert_eq!(tooltip, Some("Choose the Harvest queue to show."));

        let menu = harvest_filter_dropdown_menu(&filter_model(true))
            .expect("open harvest dropdown menu")
            .0
            .into_surface();
        let option_tooltip = menu
            .find_widget(automation_harvest_filter_dropdown_option_id("Needs Review"))
            .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

        assert_eq!(
            option_tooltip,
            Some("Files not done or ignored that do not have derivatives yet.")
        );
    }

    fn filter_model(help_tooltips_enabled: bool) -> FilterSectionViewModel {
        FilterSectionViewModel {
            sidebar_width: 240.0,
            name_filter: String::new(),
            name_filter_enabled: false,
            tag_filter: String::new(),
            tag_filter_enabled: false,
            curation: CurationFilterViewModel {
                enabled: false,
                dropdown_open: false,
                selected_scope: BrowserCurationScope::All,
                options: vec![CurationFilterOptionViewModel {
                    scope: BrowserCurationScope::All,
                    label: "All",
                }],
            },
            harvest: HarvestFilterViewModel {
                enabled: false,
                dropdown_open: true,
                selected_filter: Some(HarvestFilter::NeedsReview),
                options: vec![HarvestFilterOptionViewModel {
                    filter: HarvestFilter::NeedsReview,
                    label: "Needs Review",
                }],
                family_available: false,
                family_open: false,
                help_tooltips_enabled,
            },
            playback_type_enabled: false,
            playback_type_filters: vec![PlaybackTypeFilterToggleViewModel {
                filter: PlaybackTypeFilter::OneShot,
                label: "1-Shot",
                active: false,
            }],
            rating_enabled: false,
            rating_filters: vec![RatingFilterToggleViewModel {
                level: 0,
                label: "U",
                active: false,
            }],
            panel_height: 120.0,
        }
    }
}
