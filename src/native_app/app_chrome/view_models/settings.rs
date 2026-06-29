use std::path::PathBuf;
use wavecrate::audio::{AudioDeviceSummary, AudioHostSummary, AudioOutputConfig};
#[cfg(test)]
use wavecrate::sample_sources::config::DEFAULT_RATING_DECAY_WEEKS;

use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, NativeAppState};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct AudioSettingsSnapshot {
    pub(in crate::native_app) tab: AppSettingsTab,
    pub(in crate::native_app) trash_folder: Option<PathBuf>,
    pub(in crate::native_app) rating_decay_weeks: u16,
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
            tab: state.ui.settings.ui.app_settings_tab,
            trash_folder: state.ui.settings.persisted.trash_folder.clone(),
            rating_decay_weeks: state.ui.settings.persisted.controls.rating_decay_weeks,
            detail_label: state.audio_engine_detail_label(),
            error: state.audio.settings_error.clone(),
            audio_output_config: state.audio.output_config.clone(),
            open_dropdown: state
                .ui
                .settings
                .ui
                .audio_settings_dropdown
                .current()
                .copied(),
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

    #[cfg(test)]
    pub(in crate::native_app) fn test_default() -> Self {
        Self {
            tab: AppSettingsTab::AudioEngine,
            trash_folder: None,
            rating_decay_weeks: DEFAULT_RATING_DECAY_WEEKS,
            detail_label: "no audio".to_string(),
            error: None,
            audio_output_config: AudioOutputConfig::default(),
            open_dropdown: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn set_open_dropdown_for_tests(
        &mut self,
        dropdown: AudioSettingsDropdown,
    ) {
        self.open_dropdown = Some(dropdown);
    }
}
