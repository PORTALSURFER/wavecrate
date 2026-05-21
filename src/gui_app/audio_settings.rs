use radiant::prelude as ui;
use radiant::prelude::IntoView;
use std::sync::Arc;
use wavecrate::audio::{AudioDeviceSummary, AudioHostSummary, AudioOutputConfig};

use super::{
    AUDIO_ENGINE_PILL_HEIGHT, AUDIO_ENGINE_PILL_ID, AUDIO_ENGINE_PILL_WIDTH,
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, GuiAppState, GuiMessage,
    VOLUME_SLIDER_HEIGHT, VOLUME_SLIDER_ID, VOLUME_SLIDER_WIDTH,
};

mod audio_engine_pill;
pub(super) use audio_engine_pill::AudioEnginePill;

mod popover;
#[cfg(test)]
pub(super) use popover::audio_settings_popover;
pub(super) use popover::{audio_settings_window_view, format_sample_rate_label};

#[derive(Clone, Debug)]
pub(super) struct AudioSettingsSnapshot {
    pub(super) detail_label: String,
    pub(super) error: Option<String>,
    pub(super) audio_output_config: AudioOutputConfig,
    pub(super) audio_hosts: Vec<AudioHostSummary>,
    pub(super) audio_devices: Vec<AudioDeviceSummary>,
    pub(super) audio_sample_rates: Vec<u32>,
}

impl AudioSettingsSnapshot {
    pub(super) fn from_app_state(state: &GuiAppState) -> Self {
        Self {
            detail_label: state.audio_engine_detail_label(),
            error: state.audio_settings_error.clone(),
            audio_output_config: state.audio_output_config.clone(),
            audio_hosts: state.audio_hosts.clone(),
            audio_devices: state.audio_devices.clone(),
            audio_sample_rates: state.audio_sample_rates.clone(),
        }
    }
}

pub(super) fn auxiliary_windows(state: &mut GuiAppState) -> Vec<ui::AuxiliaryWindow<GuiMessage>> {
    if !state.audio_settings_open {
        return Vec::new();
    }
    let snapshot = AudioSettingsSnapshot::from_app_state(state);
    let options = ui::NativeRunOptions {
        title: String::from("Audio Engine"),
        inner_size: Some([AUDIO_SETTINGS_POPUP_WIDTH, AUDIO_SETTINGS_POPUP_HEIGHT]),
        min_inner_size: Some([AUDIO_SETTINGS_POPUP_WIDTH, AUDIO_SETTINGS_POPUP_HEIGHT]),
        skip_taskbar: true,
        decorations: true,
        ..ui::NativeRunOptions::default()
    };
    let surface = ui::UiSurface::new(audio_settings_window_view(&snapshot).into_node());
    vec![
        ui::AuxiliaryWindow::new("audio-settings", options, Arc::new(surface))
            .on_close(GuiMessage::CloseAudioSettings),
    ]
}

pub(super) fn top_status_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::row([
        volume_slider(state.volume),
        ui::spacer().height(20.0).fill_width(),
        audio_engine_pill(state.audio_engine_pill_label(), state.audio_settings_open),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn audio_engine_pill(label: String, active: bool) -> ui::View<GuiMessage> {
    audio_engine_pill_with_id(label, active, AUDIO_ENGINE_PILL_ID, "top-audio-engine-pill")
}

fn audio_engine_pill_with_id(
    label: String,
    active: bool,
    id: u64,
    key: &'static str,
) -> ui::View<GuiMessage> {
    ui::custom_widget(AudioEnginePill::new(label, active), |output| {
        output.typed_ref::<GuiMessage>().cloned()
    })
    .id(id)
    .key(key)
    .size(AUDIO_ENGINE_PILL_WIDTH, AUDIO_ENGINE_PILL_HEIGHT)
}

pub(super) fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
    ui::slider(volume)
        .compact()
        .message(GuiMessage::SetVolume)
        .id(VOLUME_SLIDER_ID)
        .key("top-volume-slider")
        .size(VOLUME_SLIDER_WIDTH, VOLUME_SLIDER_HEIGHT)
}
