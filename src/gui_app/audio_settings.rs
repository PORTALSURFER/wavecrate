use radiant::prelude as ui;

use super::{
    AUDIO_ENGINE_PILL_HEIGHT, AUDIO_ENGINE_PILL_ID, AUDIO_ENGINE_PILL_WIDTH,
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, GuiAppState, GuiMessage,
    VOLUME_SLIDER_HEIGHT, VOLUME_SLIDER_ID, VOLUME_SLIDER_WIDTH,
};

mod audio_engine_pill;
pub(super) use audio_engine_pill::AudioEnginePill;

mod popover;
pub(super) use popover::{audio_settings_popover, format_sample_rate_label};

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
