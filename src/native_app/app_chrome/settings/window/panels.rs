use radiant::prelude as ui;

use super::AUDIO_SETTINGS_LABELED_ROW_HEIGHT;
use super::AUDIO_SETTINGS_ROW_SPACING;
use super::dropdowns::{audio_host_dropdown, audio_output_dropdown, audio_sample_rate_dropdown};
use crate::native_app::app::{AppSettingsTab, GuiMessage, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

pub(super) fn settings_content(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let rows = match snapshot.tab {
        AppSettingsTab::General => general_settings_panel_rows(snapshot),
        AppSettingsTab::AudioEngine => audio_settings_panel_rows(snapshot),
    };
    ui::column(rows)
        .key("settings-content")
        .spacing(AUDIO_SETTINGS_ROW_SPACING)
        .fill()
}

fn audio_settings_panel_rows(snapshot: &AudioSettingsSnapshot) -> Vec<ui::View<GuiMessage>> {
    let mut rows = vec![audio_engine_detail_row(snapshot)];
    if let Some(error) = snapshot.error.as_ref() {
        rows.push(audio_settings_error_row(error));
    }
    rows.push(audio_settings_backend_section(snapshot));
    rows.push(audio_settings_labeled_control(
        "Output",
        audio_output_dropdown(snapshot),
    ));
    rows.push(audio_settings_labeled_control(
        "Sample Rate",
        audio_sample_rate_dropdown(snapshot),
    ));
    rows
}

fn general_settings_panel_rows(snapshot: &AudioSettingsSnapshot) -> Vec<ui::View<GuiMessage>> {
    vec![
        ui::text_line("General", 24.0).key("general-settings-title"),
        trash_folder_section(snapshot),
        cache_maintenance_section(),
    ]
}

fn audio_engine_detail_row(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::text_line(snapshot.detail_label.clone(), 20.0).key("audio-settings-detail")
}

fn audio_settings_error_row(error: &str) -> ui::View<GuiMessage> {
    ui::text_line(error.to_string(), 20.0)
        .key("audio-settings-error")
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Danger))
}

fn trash_folder_section(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let label = snapshot
        .trash_folder
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| String::from("No trash folder configured"));
    ui::column([
        ui::text_line("Trash Folder", 20.0).key("settings-trash-folder-label"),
        ui::text_line(label, 20.0)
            .key("settings-trash-folder-value")
            .fill_width(),
        ui::row([
            ui::button("Choose Folder")
                .message(GuiMessage::Settings(SettingsMessage::PickTrashFolder))
                .key("settings-trash-folder-pick")
                .height(24.0),
            ui::button("Clear")
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

fn cache_maintenance_section() -> ui::View<GuiMessage> {
    ui::labeled_control(
        "Maintenance",
        ui::button("Clear Rebuildable Caches")
            .message(GuiMessage::Settings(
                SettingsMessage::ClearRebuildableCaches,
            ))
            .key("settings-clear-rebuildable-caches")
            .fill_width()
            .height(24.0),
        AUDIO_SETTINGS_LABELED_ROW_HEIGHT,
    )
}

fn audio_settings_backend_section(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    audio_settings_labeled_control("Backend", audio_host_dropdown(snapshot))
}

fn audio_settings_labeled_control(
    label: &'static str,
    control: ui::View<GuiMessage>,
) -> ui::View<GuiMessage> {
    ui::labeled_control(label, control, AUDIO_SETTINGS_LABELED_ROW_HEIGHT)
}
