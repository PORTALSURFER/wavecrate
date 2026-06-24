use radiant::prelude as ui;

use super::{AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH};
#[cfg(test)]
use crate::native_app::app::NativeAppState;
use crate::native_app::app::{GuiMessage, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

mod dropdowns;
mod panels;
mod sidebar;

pub(super) const AUDIO_SETTINGS_PANEL_PADDING: f32 = 8.0;
pub(super) const AUDIO_SETTINGS_ROW_SPACING: f32 = 7.0;
pub(super) const AUDIO_SETTINGS_DROPDOWN_GAP: f32 = 3.0;
pub(super) const AUDIO_SETTINGS_LABELED_ROW_HEIGHT: f32 = 45.0;
pub(super) const SETTINGS_SIDEBAR_WIDTH: f32 = 132.0;
pub(super) const SETTINGS_CONTENT_X: f32 =
    AUDIO_SETTINGS_PANEL_PADDING + SETTINGS_SIDEBAR_WIDTH + 8.0;
pub(super) const SETTINGS_CONTENT_WIDTH: f32 =
    AUDIO_SETTINGS_POPUP_WIDTH - AUDIO_SETTINGS_PANEL_PADDING * 2.0 - SETTINGS_SIDEBAR_WIDTH - 8.0;

#[cfg(test)]
pub(in crate::native_app) fn audio_settings_popover(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    let snapshot = AudioSettingsSnapshot::from_app_state(state);
    audio_settings_window_view(&snapshot)
}

pub(in crate::native_app) fn audio_settings_window_view(
    snapshot: &AudioSettingsSnapshot,
) -> ui::View<GuiMessage> {
    let panel = ui::row([
        sidebar::settings_sidebar(snapshot),
        panels::settings_content(snapshot),
    ])
    .key("audio-settings-window")
    .style(ui::WidgetStyle::strong(ui::WidgetTone::Neutral))
    .spacing(8.0)
    .padding(AUDIO_SETTINGS_PANEL_PADDING)
    .width(AUDIO_SETTINGS_POPUP_WIDTH)
    .height(AUDIO_SETTINGS_POPUP_HEIGHT);
    let base = ui::centered_layer(
        panel,
        ui::Vector2::new(AUDIO_SETTINGS_POPUP_WIDTH, AUDIO_SETTINGS_POPUP_HEIGHT),
    );
    if snapshot.open_dropdown().is_some() {
        ui::dismissible_overlay_with_interactive_base(
            base,
            dropdowns::audio_settings_dropdown_overlay(snapshot),
            GuiMessage::Settings(SettingsMessage::CloseAudioSettingsDropdowns),
        )
    } else {
        base
    }
}
