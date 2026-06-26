use radiant::prelude as ui;
use radiant::prelude::IntoView;
use std::sync::Arc;

use crate::native_app::app::{GuiMessage, NativeAppState, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

mod top_bar;
mod window;

pub(in crate::native_app) use top_bar::top_control_bar;
#[cfg(test)]
pub(in crate::native_app) use top_bar::{
    AUDIO_ENGINE_PILL_ID, GENERAL_SETTINGS_BUTTON_ID, HELP_TOOLTIPS_BUTTON_ID,
    RELEASE_UPDATE_BUTTON_ID, VOLUME_SLIDER_ID, volume_slider,
};
#[cfg(test)]
pub(in crate::native_app) use window::audio_settings_popover;
pub(in crate::native_app) use window::audio_settings_window_view;

pub(in crate::native_app) const AUDIO_SETTINGS_POPUP_WIDTH: f32 = 520.0;
pub(in crate::native_app) const AUDIO_SETTINGS_POPUP_HEIGHT: f32 = 380.0;

pub(in crate::native_app) fn auxiliary_windows(
    state: &mut NativeAppState,
) -> Vec<ui::AuxiliaryWindow<GuiMessage>> {
    if !state.ui.settings.ui.audio_settings_open {
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
            .on_close(GuiMessage::Settings(SettingsMessage::CloseAudioSettings))
            .cache_on_close(),
    ]
}
