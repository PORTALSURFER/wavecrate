use radiant::prelude as ui;

use super::{GuiAppState, GuiMessage};

pub(super) fn main_toolbar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::row([
        ui::spacer().height(24.0).fill_width(),
        toolbar_icon_button(19, ToolbarIcon::Loop, true, state.loop_playback),
        toolbar_icon_button(20, ToolbarIcon::Play, true, state.waveform.is_playing()),
        toolbar_icon_button(21, ToolbarIcon::Stop, state.waveform.is_playing(), false),
    ])
    .padding_y(3.0)
    .style(ui::WidgetStyle::default())
    .spacing(4.0)
    .fill_width()
    .height(34.0)
}

pub(super) fn toolbar_icon_button(
    id: u64,
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::View<GuiMessage> {
    let Some(svg_icon) = ui::SvgIcon::from_svg(icon.svg()) else {
        return ui::button("")
            .message(toolbar_button_message(icon))
            .id(id)
            .size(28.0, 24.0);
    };
    ui::icon_button(svg_icon)
        .enabled(enabled)
        .active(active)
        .message(toolbar_button_message(icon))
        .id(id)
        .size(28.0, 24.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolbarIcon {
    Loop,
    Play,
    Stop,
}

impl ToolbarIcon {
    pub(super) fn svg(self) -> &'static str {
        match self {
            Self::Loop => include_str!("assets/icons/waveform_toolbar/loop.svg"),
            Self::Play => include_str!("assets/icons/waveform_toolbar/play.svg"),
            Self::Stop => include_str!("assets/icons/waveform_toolbar/stop.svg"),
        }
    }
}

fn toolbar_button_message(icon: ToolbarIcon) -> GuiMessage {
    match icon {
        ToolbarIcon::Loop => GuiMessage::ToggleLoopPlayback,
        ToolbarIcon::Play => GuiMessage::PlaySelectedSample,
        ToolbarIcon::Stop => GuiMessage::StopPlayback,
    }
}
