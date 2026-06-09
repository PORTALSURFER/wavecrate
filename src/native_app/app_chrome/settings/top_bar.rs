use radiant::prelude as ui;

use crate::native_app::app::{AppSettingsTab, GuiMessage, NativeAppState};
use crate::native_app::ui::ids as widget_ids;

pub(in crate::native_app) const VOLUME_SLIDER_ID: u64 = widget_ids::VOLUME_SLIDER_ID;
const VOLUME_SLIDER_WIDTH: f32 = 92.0;
const VOLUME_SLIDER_HEIGHT: f32 = 14.0;
pub(in crate::native_app) const AUDIO_ENGINE_PILL_ID: u64 = widget_ids::AUDIO_ENGINE_PILL_ID;
const AUDIO_ENGINE_PILL_WIDTH: f32 = 54.0;
const AUDIO_ENGINE_PILL_HEIGHT: f32 = 18.0;
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_ID: u64 =
    widget_ids::GENERAL_SETTINGS_BUTTON_ID;
const GENERAL_SETTINGS_BUTTON_WIDTH: f32 = 28.0;
const GENERAL_SETTINGS_BUTTON_HEIGHT: f32 = 24.0;

pub(in crate::native_app) fn top_control_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    ui::row([
        volume_slider(state.audio.volume),
        ui::spacer().fill_width().height(20.0),
        ui::row([
            audio_engine_pill(
                state.audio_engine_pill_label(),
                state.audio_engine_pill_style(),
                state.settings_ui.audio_settings_open
                    && state.settings_ui.app_settings_tab == AppSettingsTab::AudioEngine,
            ),
            general_settings_button(
                state.settings_ui.audio_settings_open
                    && state.settings_ui.app_settings_tab == AppSettingsTab::General,
            ),
        ])
        .spacing(4.0)
        .height(24.0),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn audio_engine_pill(label: String, style: ui::WidgetStyle, active: bool) -> ui::View<GuiMessage> {
    ui::badge(label)
        .style(style)
        .active(active)
        .message(GuiMessage::ToggleAudioSettings)
        .id(AUDIO_ENGINE_PILL_ID)
        .key("top-audio-engine-pill")
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
