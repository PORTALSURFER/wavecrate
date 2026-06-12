use radiant::prelude as ui;

use super::{
    AUDIO_SETTINGS_DROPDOWN_GAP, AUDIO_SETTINGS_LABELED_ROW_HEIGHT, AUDIO_SETTINGS_PANEL_PADDING,
    AUDIO_SETTINGS_ROW_SPACING, SETTINGS_CONTENT_WIDTH, SETTINGS_CONTENT_X,
};
use crate::native_app::app::{AudioSettingsDropdown, GuiMessage, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;
use crate::native_app::ui::display::format_sample_rate_label;

pub(super) fn audio_host_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_host_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::Backend),
    )
    .toggle_message(GuiMessage::Settings(
        SettingsMessage::ToggleAudioBackendDropdown,
    ))
    .build()
}

pub(super) fn audio_output_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_output_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::Output),
    )
    .toggle_message(GuiMessage::Settings(
        SettingsMessage::ToggleAudioOutputDropdown,
    ))
    .build()
}

pub(super) fn audio_sample_rate_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_sample_rate_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::SampleRate),
    )
    .toggle_message(GuiMessage::Settings(
        SettingsMessage::ToggleAudioSampleRateDropdown,
    ))
    .build()
}

pub(super) fn audio_settings_dropdown_overlay(
    snapshot: &AudioSettingsSnapshot,
) -> ui::View<GuiMessage> {
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
        |host| GuiMessage::Settings(SettingsMessage::SetAudioOutputHost(host)),
    )];
    for host in &snapshot.audio_hosts {
        options.push(ui::DropdownOption::for_optional_value(
            default_option_label(host.label.as_str(), host.is_default),
            Some(host.id.clone()),
            snapshot.audio_output_config.host.as_ref(),
            |host| GuiMessage::Settings(SettingsMessage::SetAudioOutputHost(host)),
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
        |device| GuiMessage::Settings(SettingsMessage::SetAudioOutputDevice(device)),
    )];
    options.extend(snapshot.audio_devices.iter().map(|device| {
        ui::DropdownOption::for_optional_value(
            default_option_label(device.name.as_str(), device.is_default),
            Some(device.name.clone()),
            snapshot.audio_output_config.device.as_ref(),
            |device| GuiMessage::Settings(SettingsMessage::SetAudioOutputDevice(device)),
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
        |sample_rate| GuiMessage::Settings(SettingsMessage::SetAudioOutputSampleRate(sample_rate)),
    )];
    options.extend(snapshot.audio_sample_rates.iter().copied().map(|rate| {
        ui::DropdownOption::for_optional_value(
            format_sample_rate_label(rate),
            Some(rate),
            snapshot.audio_output_config.sample_rate.as_ref(),
            |sample_rate| {
                GuiMessage::Settings(SettingsMessage::SetAudioOutputSampleRate(sample_rate))
            },
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

#[cfg(test)]
mod tests {
    use super::default_option_label;

    #[test]
    fn default_option_label_marks_default_items() {
        assert_eq!(default_option_label("WASAPI", true), "WASAPI (default)");
        assert_eq!(default_option_label("ASIO", false), "ASIO");
    }
}
