use std::path::PathBuf;
use wavecrate::audio::{AudioDeviceSummary, AudioHostSummary, AudioOutputConfig};

use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, NativeAppState};

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
            error: state.audio.settings_error.clone(),
            audio_output_config: state.audio.output_config.clone(),
            open_dropdown: state.audio_settings_dropdown.current().copied(),
            audio_hosts: state.audio.hosts.clone(),
            audio_devices: state.audio.devices.clone(),
            audio_sample_rates: state.audio.sample_rates.clone(),
        }
    }

    pub(in crate::native_app) fn dropdown_open(&self, dropdown: AudioSettingsDropdown) -> bool {
        self.open_dropdown == Some(dropdown)
    }

    pub(in crate::native_app) fn open_dropdown(&self) -> Option<AudioSettingsDropdown> {
        self.open_dropdown
    }
}
