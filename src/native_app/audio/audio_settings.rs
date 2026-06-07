use radiant::prelude as ui;
use radiant::prelude::IntoView;
use std::path::PathBuf;
use std::sync::Arc;
use wavecrate::audio::{AudioDeviceSummary, AudioHostSummary, AudioOutputConfig};

use crate::native_app::ui::ids as widget_ids;

pub(in crate::native_app) const VOLUME_SLIDER_ID: u64 = widget_ids::VOLUME_SLIDER_ID;
pub(in crate::native_app) const VOLUME_SLIDER_WIDTH: f32 = 92.0;
pub(in crate::native_app) const VOLUME_SLIDER_HEIGHT: f32 = 14.0;
pub(in crate::native_app) const AUDIO_ENGINE_PILL_ID: u64 = widget_ids::AUDIO_ENGINE_PILL_ID;
pub(in crate::native_app) const AUDIO_ENGINE_PILL_WIDTH: f32 = 54.0;
pub(in crate::native_app) const AUDIO_ENGINE_PILL_HEIGHT: f32 = 18.0;
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_ID: u64 =
    widget_ids::GENERAL_SETTINGS_BUTTON_ID;
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_WIDTH: f32 = 28.0;
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_HEIGHT: f32 = 24.0;
pub(in crate::native_app) const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 520.0;
pub(in crate::native_app) const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 380.0;

use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, GuiMessage, NativeAppState};

mod popover;
#[cfg(test)]
pub(in crate::native_app) use popover::audio_settings_popover;
pub(in crate::native_app) use popover::{audio_settings_window_view, format_sample_rate_label};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct AudioSettingsSnapshot {
    pub(in crate::native_app) tab: AppSettingsTab,
    pub(in crate::native_app) trash_folder: Option<PathBuf>,
    pub(in crate::native_app) detail_label: String,
    pub(in crate::native_app) error: Option<String>,
    pub(in crate::native_app) audio_output_config: AudioOutputConfig,
    open_dropdown: Option<AudioSettingsDropdown>,
    pub(in crate::native_app) audio_hosts: Vec<AudioHostSummary>,
    pub(in crate::native_app) audio_devices: Vec<AudioDeviceSummary>,
    pub(in crate::native_app) audio_sample_rates: Vec<u32>,
}

impl AudioSettingsSnapshot {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
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

    pub(in crate::native_app) fn dropdown_open(&self, dropdown: AudioSettingsDropdown) -> bool {
        self.open_dropdown == Some(dropdown)
    }

    pub(in crate::native_app) fn open_dropdown(&self) -> Option<AudioSettingsDropdown> {
        self.open_dropdown
    }
}

pub(in crate::native_app) fn auxiliary_windows(
    state: &mut NativeAppState,
) -> Vec<ui::AuxiliaryWindow<GuiMessage>> {
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
            .on_close(GuiMessage::CloseAudioSettings)
            .cache_on_close(),
    ]
}

pub(in crate::native_app) fn top_status_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
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

pub(in crate::native_app) fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
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
