use radiant::prelude as ui;

#[cfg(test)]
use super::GuiAppState;
use super::{
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, AudioSettingsDropdown,
    AudioSettingsSnapshot, GuiMessage,
};

const AUDIO_SETTINGS_PANEL_PADDING: f32 = 8.0;
const AUDIO_SETTINGS_ROW_SPACING: f32 = 7.0;
const AUDIO_SETTINGS_DROPDOWN_GAP: f32 = 3.0;
const AUDIO_SETTINGS_LABELED_ROW_HEIGHT: f32 = 45.0;

#[cfg(test)]
pub(in crate::gui_app) fn audio_settings_popover(state: &GuiAppState) -> ui::View<GuiMessage> {
    let snapshot = AudioSettingsSnapshot::from_app_state(state);
    audio_settings_window_view(&snapshot)
}

pub(in crate::gui_app) fn audio_settings_window_view(
    snapshot: &AudioSettingsSnapshot,
) -> ui::View<GuiMessage> {
    let panel = ui::column(audio_settings_panel_rows(snapshot))
        .key("audio-settings-window")
        .style(ui::WidgetStyle::strong(ui::WidgetTone::Neutral))
        .spacing(AUDIO_SETTINGS_ROW_SPACING)
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
    rows.push(cache_maintenance_section());
    rows
}

fn audio_engine_detail_row(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::text_line(snapshot.detail_label.clone(), 20.0).key("audio-settings-detail")
}

fn audio_settings_error_row(error: &str) -> ui::View<GuiMessage> {
    ui::text_line(error.to_string(), 20.0)
        .key("audio-settings-error")
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Danger))
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
        AUDIO_SETTINGS_PANEL_PADDING,
        AUDIO_SETTINGS_PANEL_PADDING,
        audio_settings_dropdown_cursor(snapshot, row_index),
        AUDIO_SETTINGS_DROPDOWN_GAP,
        Some(AUDIO_SETTINGS_POPUP_WIDTH - AUDIO_SETTINGS_PANEL_PADDING * 2.0),
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

pub(in crate::gui_app) fn format_sample_rate_label(sample_rate: u32) -> String {
    if sample_rate >= 1000 && sample_rate.is_multiple_of(1000) {
        format!("{} kHz", sample_rate / 1000)
    } else if sample_rate >= 1000 {
        format!("{:.1} kHz", sample_rate as f32 / 1000.0)
    } else {
        format!("{sample_rate} Hz")
    }
}
