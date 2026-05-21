use radiant::prelude as ui;

use super::{AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, GuiAppState, GuiMessage};

pub(in crate::gui_app) fn audio_settings_popover(state: &GuiAppState) -> ui::View<GuiMessage> {
    let panel = ui::column(audio_settings_panel_rows(state))
        .key("audio-settings-panel")
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Strong,
        })
        .spacing(7.0)
        .padding(8.0)
        .width(AUDIO_SETTINGS_POPUP_WIDTH)
        .height(AUDIO_SETTINGS_POPUP_HEIGHT);
    ui::column(vec![
        ui::spacer().height(42.0),
        ui::row(vec![ui::spacer().fill_width(), panel])
            .padding_x(14.0)
            .fill_width()
            .height(AUDIO_SETTINGS_POPUP_HEIGHT),
        ui::spacer().fill_height(),
    ])
    .fill()
}

fn audio_settings_panel_rows(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut rows = vec![audio_settings_title_row(), audio_engine_detail_row(state)];
    if let Some(error) = state.audio_settings_error.as_ref() {
        rows.push(audio_settings_error_row(error));
    }
    rows.push(audio_settings_section(
        "Backend",
        audio_host_option_buttons(state),
        2,
    ));
    rows.push(audio_settings_section(
        "Output",
        audio_device_option_buttons(state),
        2,
    ));
    rows.push(audio_settings_section(
        "Sample Rate",
        audio_sample_rate_option_buttons(state),
        4,
    ));
    rows.push(cache_maintenance_section());
    rows
}

fn audio_settings_title_row() -> ui::View<GuiMessage> {
    ui::row(vec![
        ui::text("Audio Engine").height(20.0).fill_width(),
        ui::button("x")
            .subtle()
            .message(GuiMessage::CloseAudioSettings)
            .width(24.0)
            .height(20.0),
    ])
    .fill_width()
    .height(22.0)
}

fn audio_engine_detail_row(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::text(state.audio_engine_detail_label())
        .key("audio-settings-detail")
        .fill_width()
        .height(20.0)
        .truncate()
}

fn audio_settings_error_row(error: &str) -> ui::View<GuiMessage> {
    ui::text(error.to_string())
        .key("audio-settings-error")
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Danger,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .height(20.0)
        .truncate()
}

fn cache_maintenance_section() -> ui::View<GuiMessage> {
    ui::column(vec![
        section_label("Maintenance"),
        ui::button("Clear Rebuildable Caches")
            .message(GuiMessage::ClearRebuildableCaches)
            .key("settings-clear-rebuildable-caches")
            .fill_width()
            .height(24.0),
    ])
    .spacing(3.0)
    .fill_width()
    .height(45.0)
}

fn audio_settings_section(
    label: &'static str,
    options: Vec<ui::View<GuiMessage>>,
    columns: usize,
) -> ui::View<GuiMessage> {
    let grid_height = audio_option_grid_height(options.len(), columns);
    let mut rows = vec![section_label(label)];
    if options.is_empty() {
        rows.push(ui::text("Unavailable").fill_width().height(20.0));
    } else {
        rows.push(
            ui::grid(options, columns.max(1))
                .fill_width()
                .height(grid_height),
        );
    }
    ui::column(rows)
        .spacing(3.0)
        .fill_width()
        .height(21.0 + grid_height)
}

fn section_label(label: &'static str) -> ui::View<GuiMessage> {
    ui::text(label)
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .height(18.0)
}

fn audio_host_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "System default".to_string(),
        state.audio_output_config.host.is_none(),
        GuiMessage::SetAudioOutputHost(None),
    )];
    buttons.extend(state.audio_hosts.iter().map(|host| {
        audio_option_button(
            default_option_label(host.label.as_str(), host.is_default),
            state.audio_output_config.host.as_deref() == Some(host.id.as_str()),
            GuiMessage::SetAudioOutputHost(Some(host.id.clone())),
        )
    }));
    buttons
}

fn audio_device_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "Host default".to_string(),
        state.audio_output_config.device.is_none(),
        GuiMessage::SetAudioOutputDevice(None),
    )];
    buttons.extend(state.audio_devices.iter().map(|device| {
        audio_option_button(
            default_option_label(device.name.as_str(), device.is_default),
            state.audio_output_config.device.as_deref() == Some(device.name.as_str()),
            GuiMessage::SetAudioOutputDevice(Some(device.name.clone())),
        )
    }));
    buttons
}

fn audio_sample_rate_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "Device default".to_string(),
        state.audio_output_config.sample_rate.is_none(),
        GuiMessage::SetAudioOutputSampleRate(None),
    )];
    buttons.extend(state.audio_sample_rates.iter().copied().map(|rate| {
        audio_option_button(
            format_sample_rate_label(rate),
            state.audio_output_config.sample_rate == Some(rate),
            GuiMessage::SetAudioOutputSampleRate(Some(rate)),
        )
    }));
    buttons
}

fn audio_option_button(label: String, selected: bool, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::button(label)
        .style(ui::WidgetStyle {
            tone: if selected {
                ui::WidgetTone::Accent
            } else {
                ui::WidgetTone::Neutral
            },
            prominence: if selected {
                ui::WidgetProminence::Strong
            } else {
                ui::WidgetProminence::Subtle
            },
        })
        .message(message)
        .fill_width()
        .height(20.0)
}

fn default_option_label(label: &str, is_default: bool) -> String {
    if is_default {
        format!("{label} (default)")
    } else {
        label.to_string()
    }
}

fn audio_option_grid_height(option_count: usize, columns: usize) -> f32 {
    let columns = columns.max(1);
    let rows = option_count.max(1).div_ceil(columns);
    rows as f32 * 20.0 + rows.saturating_sub(1) as f32 * 4.0
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
