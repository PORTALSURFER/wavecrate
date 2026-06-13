use radiant::prelude as ui;

use crate::native_app::app::{AudioSettingsDropdown, GuiMessage, NativeAppState, SettingsMessage};

impl NativeAppState {
    pub(super) fn apply_settings_message(
        &mut self,
        message: SettingsMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            SettingsMessage::SetVolume(volume) => self.set_volume(volume),
            SettingsMessage::ToggleAudioSettings => self.toggle_audio_settings(),
            SettingsMessage::OpenGeneralSettings => self.open_general_settings(),
            SettingsMessage::SelectSettingsTab(tab) => self.select_settings_tab(tab),
            SettingsMessage::CloseAudioSettings => self.close_audio_settings_window(),
            SettingsMessage::ToggleAudioBackendDropdown => self.toggle_audio_backend_dropdown(),
            SettingsMessage::ToggleAudioOutputDropdown => self.toggle_audio_output_dropdown(),
            SettingsMessage::ToggleAudioSampleRateDropdown => {
                self.toggle_audio_sample_rate_dropdown();
            }
            SettingsMessage::CloseAudioSettingsDropdowns => self.close_audio_settings_dropdowns(),
            SettingsMessage::SetAudioOutputHost(host) => self.set_audio_output_host(host),
            SettingsMessage::SetAudioOutputDevice(device) => self.set_audio_output_device(device),
            SettingsMessage::SetAudioOutputSampleRate(sample_rate) => {
                self.set_audio_output_sample_rate(sample_rate);
            }
            SettingsMessage::PickTrashFolder => self.pick_trash_folder(context),
            SettingsMessage::ClearTrashFolder => self.clear_trash_folder(),
            SettingsMessage::ClearRebuildableCaches => self.clear_rebuildable_caches(),
        }
    }

    fn toggle_audio_backend_dropdown(&mut self) {
        self.toggle_audio_settings_dropdown(AudioSettingsDropdown::Backend);
    }

    fn toggle_audio_output_dropdown(&mut self) {
        self.toggle_audio_settings_dropdown(AudioSettingsDropdown::Output);
    }

    fn toggle_audio_sample_rate_dropdown(&mut self) {
        self.toggle_audio_settings_dropdown(AudioSettingsDropdown::SampleRate);
    }
}
