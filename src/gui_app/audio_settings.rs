use radiant::prelude as ui;
use radiant::prelude::IntoView;
use std::path::PathBuf;
use std::sync::Arc;
use wavecrate::audio::{AudioDeviceSummary, AudioHostSummary, AudioOutputConfig};

use super::{
    AUDIO_ENGINE_PILL_HEIGHT, AUDIO_ENGINE_PILL_ID, AUDIO_ENGINE_PILL_WIDTH,
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, AppSettingsTab, AudioSettingsDropdown,
    GENERAL_SETTINGS_BUTTON_HEIGHT, GENERAL_SETTINGS_BUTTON_ID, GENERAL_SETTINGS_BUTTON_WIDTH,
    GuiAppState, GuiMessage, VOLUME_SLIDER_HEIGHT, VOLUME_SLIDER_ID, VOLUME_SLIDER_WIDTH,
};

mod popover;
#[cfg(test)]
pub(super) use popover::audio_settings_popover;
pub(super) use popover::{audio_settings_window_view, format_sample_rate_label};

#[derive(Clone, Debug)]
pub(super) struct AudioSettingsSnapshot {
    pub(super) tab: AppSettingsTab,
    pub(super) trash_folder: Option<PathBuf>,
    pub(super) detail_label: String,
    pub(super) error: Option<String>,
    pub(super) audio_output_config: AudioOutputConfig,
    open_dropdown: Option<AudioSettingsDropdown>,
    pub(super) audio_hosts: Vec<AudioHostSummary>,
    pub(super) audio_devices: Vec<AudioDeviceSummary>,
    pub(super) audio_sample_rates: Vec<u32>,
}

impl AudioSettingsSnapshot {
    pub(super) fn from_app_state(state: &GuiAppState) -> Self {
        Self {
            tab: state.app_settings_tab,
            trash_folder: state.persisted_settings.trash_folder.clone(),
            detail_label: state.audio_engine_detail_label(),
            error: state.audio_settings_error.clone(),
            audio_output_config: state.audio_output_config.clone(),
            open_dropdown: state.audio_settings_dropdown.current().copied(),
            audio_hosts: state.audio_hosts.clone(),
            audio_devices: state.audio_devices.clone(),
            audio_sample_rates: state.audio_sample_rates.clone(),
        }
    }

    pub(super) fn dropdown_open(&self, dropdown: AudioSettingsDropdown) -> bool {
        self.open_dropdown == Some(dropdown)
    }

    pub(super) fn open_dropdown(&self) -> Option<AudioSettingsDropdown> {
        self.open_dropdown
    }
}

pub(super) fn auxiliary_windows(state: &mut GuiAppState) -> Vec<ui::AuxiliaryWindow<GuiMessage>> {
    if !state.audio_settings_open {
        return Vec::new();
    }
    let snapshot = AudioSettingsSnapshot::from_app_state(state);
    let options = ui::NativeRunOptions::utility_window(
        "Settings",
        AUDIO_SETTINGS_POPUP_WIDTH,
        AUDIO_SETTINGS_POPUP_HEIGHT,
    );
    let surface = audio_settings_window_view(&snapshot).into_surface();
    vec![
        ui::AuxiliaryWindow::new("audio-settings", options, Arc::new(surface))
            .on_close(GuiMessage::CloseAudioSettings),
    ]
}

pub(super) fn top_status_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::toolbar_from_parts(
        ui::ToolbarParts::new([volume_slider(state.volume)])
            .trailing(
                ui::row([
                    audio_engine_pill(
                        state.audio_engine_pill_label(),
                        state.audio_engine_pill_style(),
                        state.audio_settings_open
                            && state.app_settings_tab == AppSettingsTab::AudioEngine,
                    ),
                    general_settings_button(
                        state.audio_settings_open
                            && state.app_settings_tab == AppSettingsTab::General,
                    ),
                ])
                .spacing(4.0)
                .height(24.0),
            )
            .spacing(8.0)
            .padding(12.0, 4.0)
            .spacer_height(20.0)
            .height(30.0),
    )
}

fn audio_engine_pill(label: String, style: ui::WidgetStyle, active: bool) -> ui::View<GuiMessage> {
    audio_engine_pill_with_id(
        label,
        style,
        active,
        AUDIO_ENGINE_PILL_ID,
        "top-audio-engine-pill",
    )
}

fn audio_engine_pill_with_id(
    label: String,
    style: ui::WidgetStyle,
    active: bool,
    id: u64,
    key: &'static str,
) -> ui::View<GuiMessage> {
    ui::badge(label)
        .style(style)
        .active(active)
        .message(GuiMessage::ToggleAudioSettings)
        .id(id)
        .key(key)
        .size(AUDIO_ENGINE_PILL_WIDTH, AUDIO_ENGINE_PILL_HEIGHT)
}

fn general_settings_button(active: bool) -> ui::View<GuiMessage> {
    ui::icon_button(settings_gear_icon(active))
        .active(active)
        .message(GuiMessage::OpenGeneralSettings)
        .id(GENERAL_SETTINGS_BUTTON_ID)
        .key("top-general-settings-button")
        .size(
            GENERAL_SETTINGS_BUTTON_WIDTH,
            GENERAL_SETTINGS_BUTTON_HEIGHT,
        )
}

pub(super) fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
    ui::slider(volume)
        .compact()
        .message(GuiMessage::SetVolume)
        .id(VOLUME_SLIDER_ID)
        .key("top-volume-slider")
        .size(VOLUME_SLIDER_WIDTH, VOLUME_SLIDER_HEIGHT)
}

fn settings_gear_icon(active: bool) -> ui::SvgIcon {
    SETTINGS_GEAR_ICON.icon(if active {
        ui::Rgba8::new(255, 160, 82, 255)
    } else {
        ui::Rgba8::new(238, 238, 238, 255)
    })
}

static SETTINGS_GEAR_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M7.2 1.5h1.6l.4 1.8c.4.1.8.3 1.2.5l1.6-1 1.1 1.1-1 1.6c.2.4.4.8.5 1.2l1.8.4v1.6l-1.8.4c-.1.4-.3.8-.5 1.2l1 1.6-1.1 1.1-1.6-1c-.4.2-.8.4-1.2.5l-.4 1.8H7.2l-.4-1.8c-.4-.1-.8-.3-1.2-.5l-1.6 1-1.1-1.1 1-1.6c-.2-.4-.4-.8-.5-1.2l-1.8-.4V7.2l1.8-.4c.1-.4.3-.8.5-1.2l-1-1.6L4 2.9l1.6 1c.4-.2.8-.4 1.2-.5z"/>
  <circle cx="8" cy="8" r="2.2" fill="none" stroke="currentColor" stroke-width="1.4"/>
</svg>"#,
);
