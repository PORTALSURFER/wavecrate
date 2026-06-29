use radiant::prelude as ui;

mod projection;

use self::projection::{
    CacheMaintenanceProjection, RatingDecayProjection, SettingsActionProjection,
    SettingsPanelRowProjection, TrashFolderProjection, settings_panel_projection,
};
use super::AUDIO_SETTINGS_LABELED_ROW_HEIGHT;
use super::AUDIO_SETTINGS_ROW_SPACING;
use super::dropdowns::{audio_host_dropdown, audio_output_dropdown, audio_sample_rate_dropdown};
use crate::native_app::app::{AudioSettingsDropdown, GuiMessage, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

pub(super) fn settings_content(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let rows = settings_panel_projection(snapshot)
        .rows()
        .into_iter()
        .map(|row| settings_panel_row(snapshot, row))
        .collect::<Vec<_>>();
    ui::column(rows).spacing(AUDIO_SETTINGS_ROW_SPACING).fill()
}

fn settings_panel_row(
    snapshot: &AudioSettingsSnapshot,
    row: SettingsPanelRowProjection,
) -> ui::View<GuiMessage> {
    match row {
        SettingsPanelRowProjection::Title { label } => ui::text_line(label, 24.0),
        SettingsPanelRowProjection::AudioDetail { label } => audio_engine_detail_row(label),
        SettingsPanelRowProjection::AudioError { message } => audio_settings_error_row(message),
        SettingsPanelRowProjection::AudioDropdown { label, dropdown } => {
            audio_settings_dropdown_row(label, dropdown, snapshot)
        }
        SettingsPanelRowProjection::TrashFolder(projection) => trash_folder_section(projection),
        SettingsPanelRowProjection::RatingDecay(projection) => rating_decay_section(projection),
        SettingsPanelRowProjection::CacheMaintenance(projection) => {
            cache_maintenance_section(projection)
        }
    }
}

fn audio_engine_detail_row(detail_label: String) -> ui::View<GuiMessage> {
    ui::text_line(detail_label, 20.0)
}

fn audio_settings_error_row(error: String) -> ui::View<GuiMessage> {
    ui::text_line(error, 20.0).style(ui::WidgetStyle::subtle(ui::WidgetTone::Danger))
}

fn trash_folder_section(projection: TrashFolderProjection) -> ui::View<GuiMessage> {
    ui::column([
        ui::text_line(projection.label, 20.0),
        ui::text_line(projection.value, 20.0).fill_width(),
        ui::row([
            settings_action_button(projection.choose_action),
            settings_action_button(projection.clear_action),
        ])
        .spacing(6.0)
        .fill_width()
        .height(26.0),
    ])
    .spacing(4.0)
    .fill_width()
    .height(72.0)
}

fn cache_maintenance_section(projection: CacheMaintenanceProjection) -> ui::View<GuiMessage> {
    ui::labeled_control(
        projection.label,
        settings_action_button(projection.clear_action).fill_width(),
        AUDIO_SETTINGS_LABELED_ROW_HEIGHT,
    )
}

fn rating_decay_section(projection: RatingDecayProjection) -> ui::View<GuiMessage> {
    let value_label = ui::text_line(projection.value_label, 18.0).width(92.0);
    let slider = ui::slider(projection.slider_value)
        .message(|value| {
            GuiMessage::Settings(SettingsMessage::SetRatingDecayWeeks(
                RatingDecayProjection::weeks_from_slider_value(value),
            ))
        })
        .fill_width();
    let control = ui::row([slider, value_label])
        .spacing(8.0)
        .fill_width()
        .height(24.0);
    ui::labeled_control(projection.label, control, AUDIO_SETTINGS_LABELED_ROW_HEIGHT)
}

fn settings_action_button(action: SettingsActionProjection) -> ui::View<GuiMessage> {
    ui::button(action.label)
        .message(GuiMessage::Settings(action.message))
        .height(24.0)
}

fn audio_settings_dropdown_row(
    label: &'static str,
    dropdown: AudioSettingsDropdown,
    snapshot: &AudioSettingsSnapshot,
) -> ui::View<GuiMessage> {
    let control = match dropdown {
        AudioSettingsDropdown::Backend => audio_host_dropdown(snapshot),
        AudioSettingsDropdown::Output => audio_output_dropdown(snapshot),
        AudioSettingsDropdown::SampleRate => audio_sample_rate_dropdown(snapshot),
    };
    audio_settings_labeled_control(label, control)
}

fn audio_settings_labeled_control(
    label: &'static str,
    control: ui::View<GuiMessage>,
) -> ui::View<GuiMessage> {
    ui::labeled_control(label, control, AUDIO_SETTINGS_LABELED_ROW_HEIGHT)
}
