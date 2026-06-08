use radiant::prelude as ui;

use super::{AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH};
#[cfg(test)]
use crate::native_app::app::NativeAppState;
use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, GuiMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;
use crate::native_app::ui::display::format_sample_rate_label;

const AUDIO_SETTINGS_PANEL_PADDING: f32 = 8.0;
const AUDIO_SETTINGS_ROW_SPACING: f32 = 7.0;
const AUDIO_SETTINGS_DROPDOWN_GAP: f32 = 3.0;
const AUDIO_SETTINGS_LABELED_ROW_HEIGHT: f32 = 45.0;
const SETTINGS_SIDEBAR_WIDTH: f32 = 132.0;
const SETTINGS_CONTENT_X: f32 = AUDIO_SETTINGS_PANEL_PADDING + SETTINGS_SIDEBAR_WIDTH + 8.0;
const SETTINGS_CONTENT_WIDTH: f32 =
    AUDIO_SETTINGS_POPUP_WIDTH - AUDIO_SETTINGS_PANEL_PADDING * 2.0 - SETTINGS_SIDEBAR_WIDTH - 8.0;

#[cfg(test)]
pub(in crate::native_app) fn audio_settings_popover(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    let snapshot = AudioSettingsSnapshot::from_app_state(state);
    audio_settings_window_view(&snapshot)
}

pub(in crate::native_app) fn audio_settings_window_view(
    snapshot: &AudioSettingsSnapshot,
) -> ui::View<GuiMessage> {
    let panel = ui::row([settings_sidebar(snapshot), settings_content(snapshot)])
        .key("audio-settings-window")
        .style(ui::WidgetStyle::strong(ui::WidgetTone::Neutral))
        .spacing(8.0)
        .padding(AUDIO_SETTINGS_PANEL_PADDING)
        .width(AUDIO_SETTINGS_POPUP_WIDTH)
        .height(AUDIO_SETTINGS_POPUP_HEIGHT);
    let base = ui::centered_layer(
        panel,
        ui::Vector2::new(AUDIO_SETTINGS_POPUP_WIDTH, AUDIO_SETTINGS_POPUP_HEIGHT),
    );
    if snapshot.open_dropdown().is_some() {
        ui::dismissible_overlay(
            base,
            audio_settings_dropdown_overlay(snapshot),
            GuiMessage::CloseAudioSettingsDropdowns,
        )
    } else {
        base
    }
}

fn settings_sidebar(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::column([
        ui::text_line("Settings", 24.0).key("settings-sidebar-title"),
        settings_tab_button("General", AppSettingsTab::General, snapshot.tab),
        settings_tab_button("Audio Engine", AppSettingsTab::AudioEngine, snapshot.tab),
        ui::spacer().fill_width().fill_height(),
    ])
    .key("settings-sidebar")
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
    .padding(6.0)
    .spacing(4.0)
    .width(SETTINGS_SIDEBAR_WIDTH)
    .fill_height()
}

fn settings_tab_button(
    label: &'static str,
    tab: AppSettingsTab,
    selected: AppSettingsTab,
) -> ui::View<GuiMessage> {
    let style = if tab == selected {
        ui::WidgetStyle::strong(ui::WidgetTone::Accent)
    } else {
        ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)
    };
    ui::button(label)
        .style(style)
        .message(GuiMessage::SelectSettingsTab(tab))
        .key(format!("settings-tab-{label}"))
        .fill_width()
        .height(28.0)
}

fn settings_content(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
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
                .message(GuiMessage::PickTrashFolder)
                .key("settings-trash-folder-pick")
                .height(24.0),
            ui::button("Clear")
                .message(GuiMessage::ClearTrashFolder)
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
            .message(GuiMessage::ClearRebuildableCaches)
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

fn audio_host_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_host_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::Backend),
    )
    .toggle_message(GuiMessage::ToggleAudioBackendDropdown)
    .build()
}

fn audio_output_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_output_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::Output),
    )
    .toggle_message(GuiMessage::ToggleAudioOutputDropdown)
    .build()
}

fn audio_sample_rate_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_sample_rate_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::SampleRate),
    )
    .toggle_message(GuiMessage::ToggleAudioSampleRateDropdown)
    .build()
}

fn audio_settings_dropdown_overlay(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let Some((row_index, options)) = audio_settings_open_dropdown_options(snapshot) else {
        return ui::empty().fill_width();
    };
    ui::dropdown_menu_overlay_below_stacked_labeled_control(
        SETTINGS_CONTENT_X,
        AUDIO_SETTINGS_PANEL_PADDING,
        audio_settings_dropdown_cursor(snapshot, row_index),
        AUDIO_SETTINGS_DROPDOWN_GAP,
        Some(SETTINGS_CONTENT_WIDTH),
        options,
    )
}

fn audio_settings_open_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Option<(usize, Vec<ui::DropdownOption<GuiMessage>>)> {
    match snapshot.open_dropdown()? {
        AudioSettingsDropdown::Backend => Some((0, audio_host_dropdown_options(snapshot))),
        AudioSettingsDropdown::Output => Some((1, audio_output_dropdown_options(snapshot))),
        AudioSettingsDropdown::SampleRate => {
            Some((2, audio_sample_rate_dropdown_options(snapshot)))
        }
    }
}

fn audio_settings_dropdown_cursor(
    snapshot: &AudioSettingsSnapshot,
    labeled_rows_before: usize,
) -> ui::StackedLayoutCursor {
    ui::StackedLayoutCursor::new()
        .advanced(20.0, AUDIO_SETTINGS_ROW_SPACING)
        .advanced_if(snapshot.error.is_some(), 20.0, AUDIO_SETTINGS_ROW_SPACING)
        .advanced_many(
            labeled_rows_before,
            AUDIO_SETTINGS_LABELED_ROW_HEIGHT,
            AUDIO_SETTINGS_ROW_SPACING,
        )
}

fn audio_host_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<ui::DropdownOption<GuiMessage>> {
    let mut options = vec![ui::DropdownOption::for_optional_value(
        "System default",
        None::<String>,
        snapshot.audio_output_config.host.as_ref(),
        GuiMessage::SetAudioOutputHost,
    )];
    for host in &snapshot.audio_hosts {
        options.push(ui::DropdownOption::for_optional_value(
            default_option_label(host.label.as_str(), host.is_default),
            Some(host.id.clone()),
            snapshot.audio_output_config.host.as_ref(),
            GuiMessage::SetAudioOutputHost,
        ));
    }
    options
}

fn selected_audio_host_label(snapshot: &AudioSettingsSnapshot) -> String {
    snapshot
        .audio_output_config
        .host
        .as_deref()
        .and_then(|selected| {
            snapshot
                .audio_hosts
                .iter()
                .find(|host| host.id == selected)
                .map(|host| default_option_label(host.label.as_str(), host.is_default))
        })
        .or_else(|| snapshot.audio_output_config.host.clone())
        .unwrap_or_else(|| String::from("System default"))
}

fn audio_output_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<ui::DropdownOption<GuiMessage>> {
    let mut options = vec![ui::DropdownOption::for_optional_value(
        "Host default",
        None::<String>,
        snapshot.audio_output_config.device.as_ref(),
        GuiMessage::SetAudioOutputDevice,
    )];
    options.extend(snapshot.audio_devices.iter().map(|device| {
        ui::DropdownOption::for_optional_value(
            default_option_label(device.name.as_str(), device.is_default),
            Some(device.name.clone()),
            snapshot.audio_output_config.device.as_ref(),
            GuiMessage::SetAudioOutputDevice,
        )
    }));
    options
}

fn selected_audio_output_label(snapshot: &AudioSettingsSnapshot) -> String {
    snapshot
        .audio_output_config
        .device
        .as_deref()
        .and_then(|selected| {
            snapshot
                .audio_devices
                .iter()
                .find(|device| device.name == selected)
                .map(|device| default_option_label(device.name.as_str(), device.is_default))
        })
        .or_else(|| snapshot.audio_output_config.device.clone())
        .unwrap_or_else(|| String::from("Host default"))
}

fn audio_sample_rate_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<ui::DropdownOption<GuiMessage>> {
    let mut options = vec![ui::DropdownOption::for_optional_value(
        "Device default",
        None::<u32>,
        snapshot.audio_output_config.sample_rate.as_ref(),
        GuiMessage::SetAudioOutputSampleRate,
    )];
    options.extend(snapshot.audio_sample_rates.iter().copied().map(|rate| {
        ui::DropdownOption::for_optional_value(
            format_sample_rate_label(rate),
            Some(rate),
            snapshot.audio_output_config.sample_rate.as_ref(),
            GuiMessage::SetAudioOutputSampleRate,
        )
    }));
    options
}

fn selected_audio_sample_rate_label(snapshot: &AudioSettingsSnapshot) -> String {
    snapshot
        .audio_output_config
        .sample_rate
        .map(format_sample_rate_label)
        .unwrap_or_else(|| String::from("Device default"))
}

fn default_option_label(label: &str, is_default: bool) -> String {
    if is_default {
        format!("{label} (default)")
    } else {
        label.to_string()
    }
}
