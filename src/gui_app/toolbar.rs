use radiant::prelude as ui;

use super::{GuiAppState, GuiMessage};

const TOOLBAR_ICON_ACTIVE_COLOR: &str = "#ffa052";
const TOOLBAR_ICON_ENABLED_COLOR: &str = "#eeeeee";
const TOOLBAR_ICON_DISABLED_COLOR: &str = "#919191";

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
    let svg = toolbar_icon_svg(icon, enabled, active);
    let Some(svg_icon) = ui::SvgIcon::from_svg(&svg) else {
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

pub(super) fn toolbar_icon_svg(icon: ToolbarIcon, enabled: bool, active: bool) -> String {
    let color = if !enabled {
        TOOLBAR_ICON_DISABLED_COLOR
    } else if active {
        TOOLBAR_ICON_ACTIVE_COLOR
    } else {
        TOOLBAR_ICON_ENABLED_COLOR
    };
    with_svg_current_color(icon.svg(), color)
}

fn with_svg_current_color(svg: &str, color: &str) -> String {
    let Some(index) = svg.find("<svg") else {
        return svg.to_string();
    };
    let insert_at = index + "<svg".len();
    format!(
        "{} fill=\"{}\"{}",
        &svg[..insert_at],
        color,
        &svg[insert_at..]
    )
}

fn toolbar_button_message(icon: ToolbarIcon) -> GuiMessage {
    match icon {
        ToolbarIcon::Loop => GuiMessage::ToggleLoopPlayback,
        ToolbarIcon::Play => GuiMessage::PlaySelectedSample,
        ToolbarIcon::Stop => GuiMessage::StopPlayback,
    }
}
