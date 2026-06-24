use radiant::prelude as ui;

mod projection;

use self::projection::{
    AudioSettingsPanelProjection, CacheMaintenanceProjection, GeneralSettingsPanelProjection,
    SettingsPanelProjection, TrashFolderProjection, settings_panel_projection,
};
use super::AUDIO_SETTINGS_LABELED_ROW_HEIGHT;
use super::AUDIO_SETTINGS_ROW_SPACING;
use super::dropdowns::{audio_host_dropdown, audio_output_dropdown, audio_sample_rate_dropdown};
use crate::native_app::app::{GuiMessage, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

pub(super) fn settings_content(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let rows = match settings_panel_projection(snapshot) {
        SettingsPanelProjection::General(projection) => general_settings_panel_rows(projection),
        SettingsPanelProjection::AudioEngine(projection) => {
            audio_settings_panel_rows(snapshot, projection)
        }
    };
    ui::column(rows)
        .key("settings-content")
        .spacing(AUDIO_SETTINGS_ROW_SPACING)
        .fill()
}

fn audio_settings_panel_rows(
    snapshot: &AudioSettingsSnapshot,
    projection: AudioSettingsPanelProjection,
) -> Vec<ui::View<GuiMessage>> {
    let mut rows = vec![audio_engine_detail_row(projection.detail_label)];
    if let Some(error) = projection.error {
        rows.push(audio_settings_error_row(error));
    }
    rows.push(audio_settings_backend_section(
        projection.backend_label,
        snapshot,
    ));
    rows.push(audio_settings_labeled_control(
        projection.output_label,
        audio_output_dropdown(snapshot),
    ));
    rows.push(audio_settings_labeled_control(
        projection.sample_rate_label,
        audio_sample_rate_dropdown(snapshot),
    ));
    rows
}

fn general_settings_panel_rows(
    projection: GeneralSettingsPanelProjection,
) -> Vec<ui::View<GuiMessage>> {
    vec![
        ui::text_line(projection.title, 24.0).key("general-settings-title"),
        trash_folder_section(projection.trash_folder),
        cache_maintenance_section(projection.maintenance),
    ]
}

fn audio_engine_detail_row(detail_label: String) -> ui::View<GuiMessage> {
    ui::text_line(detail_label, 20.0).key("audio-settings-detail")
}

fn audio_settings_error_row(error: String) -> ui::View<GuiMessage> {
    ui::text_line(error, 20.0)
        .key("audio-settings-error")
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Danger))
}

fn trash_folder_section(projection: TrashFolderProjection) -> ui::View<GuiMessage> {
    ui::column([
        ui::text_line(projection.label, 20.0).key("settings-trash-folder-label"),
        ui::text_line(projection.value, 20.0)
            .key("settings-trash-folder-value")
            .fill_width(),
        ui::row([
            ui::button(projection.choose_button_label)
                .message(GuiMessage::Settings(SettingsMessage::PickTrashFolder))
                .key("settings-trash-folder-pick")
                .height(24.0),
            ui::button(projection.clear_button_label)
                .message(GuiMessage::Settings(SettingsMessage::ClearTrashFolder))
                .key("settings-trash-folder-clear")
                .height(24.0),
        ])
        .spacing(6.0)
        .fill_width()
        .height(26.0),
    ])
    .key("settings-trash-folder-section")
    .spacing(4.0)
    .fill_width()
    .height(72.0)
}

fn cache_maintenance_section(projection: CacheMaintenanceProjection) -> ui::View<GuiMessage> {
    ui::labeled_control(
        projection.label,
        ui::button(projection.clear_button_label)
            .message(GuiMessage::Settings(
                SettingsMessage::ClearRebuildableCaches,
            ))
            .key("settings-clear-rebuildable-caches")
            .fill_width()
            .height(24.0),
        AUDIO_SETTINGS_LABELED_ROW_HEIGHT,
    )
}

fn audio_settings_backend_section(
    label: &'static str,
    snapshot: &AudioSettingsSnapshot,
) -> ui::View<GuiMessage> {
    audio_settings_labeled_control(label, audio_host_dropdown(snapshot))
}

fn audio_settings_labeled_control(
    label: &'static str,
    control: ui::View<GuiMessage>,
) -> ui::View<GuiMessage> {
    ui::labeled_control(label, control, AUDIO_SETTINGS_LABELED_ROW_HEIGHT)
}
