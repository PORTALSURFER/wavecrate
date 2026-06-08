use radiant::prelude as ui;
use radiant::prelude::IntoView;
use std::{path::PathBuf, sync::Arc};
use wavecrate::audio::{AudioDeviceSummary, AudioHostSummary, AudioOutputConfig};

use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, GuiMessage, NativeAppState};

mod top_bar;
mod window;

pub(in crate::native_app) use top_bar::top_status_bar;
#[cfg(test)]
pub(in crate::native_app) use top_bar::{
    AUDIO_ENGINE_PILL_ID, GENERAL_SETTINGS_BUTTON_ID, VOLUME_SLIDER_ID, volume_slider,
};
#[cfg(test)]
pub(in crate::native_app) use window::audio_settings_popover;
pub(in crate::native_app) use window::audio_settings_window_view;

pub(in crate::native_app) const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 520.0;
pub(in crate::native_app) const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 380.0;

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
