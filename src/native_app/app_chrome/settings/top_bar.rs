use radiant::prelude as ui;

use crate::native_app::app::{AppSettingsTab, GuiMessage, NativeAppState, SettingsMessage};
use crate::native_app::ui::ids as widget_ids;

pub(in crate::native_app) const VOLUME_SLIDER_ID: u64 = widget_ids::VOLUME_SLIDER_ID;
const VOLUME_SLIDER_SIZE: ControlSize = ControlSize {
    width: 92.0,
    height: 14.0,
};
pub(in crate::native_app) const AUDIO_ENGINE_PILL_ID: u64 = widget_ids::AUDIO_ENGINE_PILL_ID;
const AUDIO_ENGINE_PILL_SIZE: ControlSize = ControlSize {
    width: 54.0,
    height: 18.0,
};
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_ID: u64 =
    widget_ids::GENERAL_SETTINGS_BUTTON_ID;
const GENERAL_SETTINGS_BUTTON_SIZE: ControlSize = ControlSize {
    width: 28.0,
    height: 24.0,
};

struct ControlSize {
    width: f32,
    height: f32,
}

pub(in crate::native_app) fn top_control_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    let model = TopControlBarModel::from_app_state(state);
    ui::row([
        volume_slider(model.volume),
        ui::spacer().fill_width().height(20.0),
        settings_controls(model.settings_controls),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

struct TopControlBarModel {
    volume: f32,
    settings_controls: SettingsControlsModel,
}

struct SettingsControlsModel {
    audio_engine: AudioEnginePillModel,
    general_settings_active: bool,
}

struct AudioEnginePillModel {
    label: String,
    style: ui::WidgetStyle,
    active: bool,
}

impl TopControlBarModel {
    fn from_app_state(state: &NativeAppState) -> Self {
        let settings_window = &state.ui.settings.ui;
        let active_settings_tab = settings_window
            .audio_settings_open
            .then_some(settings_window.app_settings_tab);
        Self {
            volume: state.audio.volume,
            settings_controls: SettingsControlsModel {
                audio_engine: AudioEnginePillModel {
                    label: state.audio_engine_pill_label(),
                    style: state.audio_engine_pill_style(),
                    active: active_settings_tab == Some(AppSettingsTab::AudioEngine),
                },
                general_settings_active: active_settings_tab == Some(AppSettingsTab::General),
            },
        }
    }
}

fn settings_controls(model: SettingsControlsModel) -> ui::View<GuiMessage> {
    ui::row([
        audio_engine_pill(model.audio_engine),
        general_settings_button(model.general_settings_active),
    ])
    .spacing(4.0)
    .height(24.0)
}

fn audio_engine_pill(model: AudioEnginePillModel) -> ui::View<GuiMessage> {
    ui::badge(model.label)
        .style(model.style)
        .active(model.active)
        .message(GuiMessage::Settings(SettingsMessage::ToggleAudioSettings))
        .id(AUDIO_ENGINE_PILL_ID)
        .key("top-audio-engine-pill")
        .size(AUDIO_ENGINE_PILL_SIZE.width, AUDIO_ENGINE_PILL_SIZE.height)
}

fn general_settings_button(active: bool) -> ui::View<GuiMessage> {
    ui::icon_button(settings_gear_icon(active))
        .active(active)
        .message(GuiMessage::Settings(SettingsMessage::OpenGeneralSettings))
        .id(GENERAL_SETTINGS_BUTTON_ID)
        .key("top-general-settings-button")
        .size(
            GENERAL_SETTINGS_BUTTON_SIZE.width,
            GENERAL_SETTINGS_BUTTON_SIZE.height,
        )
}

pub(in crate::native_app) fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
    ui::slider(volume)
        .compact()
        .message(|volume| GuiMessage::Settings(SettingsMessage::SetVolume(volume)))
        .id(VOLUME_SLIDER_ID)
        .key("top-volume-slider")
        .size(VOLUME_SLIDER_SIZE.width, VOLUME_SLIDER_SIZE.height)
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
