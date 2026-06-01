use radiant::prelude as ui;

#[cfg(test)]
use super::GuiAppState;
use super::{
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, AudioSettingsDropdown,
    AudioSettingsSnapshot, GuiMessage,
};

const AUDIO_SETTINGS_PANEL_PADDING: f32 = 8.0;
const AUDIO_SETTINGS_ROW_SPACING: f32 = 7.0;
const AUDIO_SETTINGS_SECTION_SPACING: f32 = 3.0;
const AUDIO_SETTINGS_DROPDOWN_GAP: f32 = 3.0;

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
        .style(ui::WidgetStyle::new(
            ui::WidgetTone::Neutral,
            ui::WidgetProminence::Strong,
        ))
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
    rows.push(ui::labeled_control(
        "Output",
        audio_output_dropdown(snapshot),
        45.0,
    ));
    rows.push(ui::labeled_control(
        "Sample Rate",
        audio_sample_rate_dropdown(snapshot),
        45.0,
    ));
    rows.push(cache_maintenance_section());
    rows
}

fn audio_engine_detail_row(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::text(snapshot.detail_label.clone())
        .key("audio-settings-detail")
        .fill_width()
        .height(20.0)
        .truncate()
}

fn audio_settings_error_row(error: &str) -> ui::View<GuiMessage> {
    ui::text(error.to_string())
        .key("audio-settings-error")
        .style(ui::WidgetStyle::new(
            ui::WidgetTone::Danger,
            ui::WidgetProminence::Subtle,
        ))
        .fill_width()
        .height(20.0)
        .truncate()
}

fn cache_maintenance_section() -> ui::View<GuiMessage> {
    ui::labeled_control(
        "Maintenance",
        ui::button("Clear Rebuildable Caches")
            .message(GuiMessage::ClearRebuildableCaches)
            .key("settings-clear-rebuildable-caches")
            .fill_width()
            .height(24.0),
        45.0,
    )
}

fn audio_settings_backend_section(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::labeled_control("Backend", audio_host_dropdown(snapshot), 45.0)
}

fn audio_host_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_host_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::Backend),
    )
    .toggle_message(GuiMessage::ToggleAudioBackendDropdown)
    .build()
}

fn audio_host_dropdown_overlay(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_menu_overlay_below_trigger(
        AUDIO_SETTINGS_PANEL_PADDING,
        AUDIO_SETTINGS_PANEL_PADDING + audio_host_dropdown_y(snapshot),
        AUDIO_SETTINGS_DROPDOWN_GAP,
        Some(AUDIO_SETTINGS_POPUP_WIDTH - AUDIO_SETTINGS_PANEL_PADDING * 2.0),
        audio_host_dropdown_options(snapshot),
    )
}

fn audio_output_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_output_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::Output),
    )
    .toggle_message(GuiMessage::ToggleAudioOutputDropdown)
    .build()
}

fn audio_output_dropdown_overlay(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_menu_overlay_below_trigger(
        AUDIO_SETTINGS_PANEL_PADDING,
        AUDIO_SETTINGS_PANEL_PADDING + audio_output_dropdown_y(snapshot),
        AUDIO_SETTINGS_DROPDOWN_GAP,
        Some(AUDIO_SETTINGS_POPUP_WIDTH - AUDIO_SETTINGS_PANEL_PADDING * 2.0),
        audio_output_dropdown_options(snapshot),
    )
}

fn audio_sample_rate_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_trigger(
        selected_audio_sample_rate_label(snapshot),
        snapshot.dropdown_open(AudioSettingsDropdown::SampleRate),
    )
    .toggle_message(GuiMessage::ToggleAudioSampleRateDropdown)
    .build()
}

fn audio_sample_rate_dropdown_overlay(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::dropdown_menu_overlay_below_trigger(
        AUDIO_SETTINGS_PANEL_PADDING,
        AUDIO_SETTINGS_PANEL_PADDING + audio_sample_rate_dropdown_y(snapshot),
        AUDIO_SETTINGS_DROPDOWN_GAP,
        Some(AUDIO_SETTINGS_POPUP_WIDTH - AUDIO_SETTINGS_PANEL_PADDING * 2.0),
        audio_sample_rate_dropdown_options(snapshot),
    )
}

fn audio_settings_dropdown_overlay(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    match snapshot.open_dropdown() {
        Some(AudioSettingsDropdown::Backend) => audio_host_dropdown_overlay(snapshot),
        Some(AudioSettingsDropdown::Output) => audio_output_dropdown_overlay(snapshot),
        Some(AudioSettingsDropdown::SampleRate) => audio_sample_rate_dropdown_overlay(snapshot),
        None => ui::spacer().height(0.0).fill_width(),
    }
}

fn audio_host_dropdown_y(snapshot: &AudioSettingsSnapshot) -> f32 {
    ui::StackedLayoutCursor::new()
        .advanced(20.0, AUDIO_SETTINGS_ROW_SPACING)
        .advanced_if(snapshot.error.is_some(), 20.0, AUDIO_SETTINGS_ROW_SPACING)
        .advanced(
            ui::labeled_control_control_offset(),
            AUDIO_SETTINGS_SECTION_SPACING,
        )
        .offset()
}

fn audio_output_dropdown_y(snapshot: &AudioSettingsSnapshot) -> f32 {
    ui::StackedLayoutCursor::from_offset(audio_host_dropdown_y(snapshot))
        .advanced(45.0, AUDIO_SETTINGS_ROW_SPACING)
        .offset()
}

fn audio_sample_rate_dropdown_y(snapshot: &AudioSettingsSnapshot) -> f32 {
    ui::StackedLayoutCursor::from_offset(audio_output_dropdown_y(snapshot))
        .advanced(45.0, AUDIO_SETTINGS_ROW_SPACING)
        .offset()
}

fn audio_host_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<ui::DropdownOption<GuiMessage>> {
    let mut options = vec![ui::DropdownOption::new(
        "System default",
        snapshot.audio_output_config.host.is_none(),
        GuiMessage::SetAudioOutputHost(None),
    )];
    for host in &snapshot.audio_hosts {
        options.push(ui::DropdownOption::new(
            default_option_label(host.label.as_str(), host.is_default),
            snapshot.audio_output_config.host.as_deref() == Some(host.id.as_str()),
            GuiMessage::SetAudioOutputHost(Some(host.id.clone())),
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
    let mut options = vec![ui::DropdownOption::new(
        "Host default",
        snapshot.audio_output_config.device.is_none(),
        GuiMessage::SetAudioOutputDevice(None),
    )];
    options.extend(snapshot.audio_devices.iter().map(|device| {
        ui::DropdownOption::new(
            default_option_label(device.name.as_str(), device.is_default),
            snapshot.audio_output_config.device.as_deref() == Some(device.name.as_str()),
            GuiMessage::SetAudioOutputDevice(Some(device.name.clone())),
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
    let mut options = vec![ui::DropdownOption::new(
        "Device default",
        snapshot.audio_output_config.sample_rate.is_none(),
        GuiMessage::SetAudioOutputSampleRate(None),
    )];
    options.extend(snapshot.audio_sample_rates.iter().copied().map(|rate| {
        ui::DropdownOption::new(
            format_sample_rate_label(rate),
            snapshot.audio_output_config.sample_rate == Some(rate),
            GuiMessage::SetAudioOutputSampleRate(Some(rate)),
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
