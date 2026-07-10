use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::settings as chrome_settings;
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;
use radiant::{prelude as ui, runtime as runtime_ui};

pub(in crate::native_app) const AUDIO_ENGINE_PILL_ID: u64 = chrome_settings::AUDIO_ENGINE_PILL_ID;
pub(in crate::native_app) const AUDIO_SETTINGS_POPUP_HEIGHT: f32 =
    chrome_settings::AUDIO_SETTINGS_POPUP_HEIGHT;
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_ID: u64 =
    chrome_settings::GENERAL_SETTINGS_BUTTON_ID;
pub(in crate::native_app) const HELP_TOOLTIPS_BUTTON_ID: u64 =
    chrome_settings::HELP_TOOLTIPS_BUTTON_ID;
pub(in crate::native_app) const NORMALIZED_AUDITION_BUTTON_ID: u64 =
    chrome_settings::NORMALIZED_AUDITION_BUTTON_ID;
pub(in crate::native_app) const RELEASE_UPDATE_BUTTON_ID: u64 =
    chrome_settings::RELEASE_UPDATE_BUTTON_ID;
pub(in crate::native_app) const VOLUME_SLIDER_ID: u64 = chrome_settings::VOLUME_SLIDER_ID;

pub(in crate::native_app) fn top_control_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    chrome_settings::top_control_bar(state)
}

pub(in crate::native_app) fn audio_settings_popover(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    chrome_settings::audio_settings_popover(state)
}

pub(in crate::native_app) fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
    chrome_settings::volume_slider(volume)
}

pub(in crate::native_app) fn auxiliary_windows(
    state: &mut NativeAppState,
) -> Vec<runtime_ui::AuxiliaryWindow<GuiMessage>> {
    chrome_settings::auxiliary_windows(state)
}

pub(in crate::native_app) fn audio_settings_host_ids(state: &NativeAppState) -> Vec<String> {
    AudioSettingsSnapshot::from_app_state(state)
        .audio_hosts
        .iter()
        .map(|host| host.id.clone())
        .collect()
}
