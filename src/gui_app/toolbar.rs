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
    pub(super) fn svg(self, color: &str) -> String {
        match self {
            Self::Loop => format!(
                r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill="{color}" d="M4 3h5.4V1.5L14 5l-4.6 3.5V7H4.2C3 7 2 8 2 9.2V10H.5v-.8C.5 5.8 2 3 4 3z"/>
  <path fill="{color}" d="M12 13H6.6v1.5L2 11l4.6-3.5V9H12c1.2 0 2-1 2-2.2V6h1.5v.8C15.5 10.2 14 13 12 13z"/>
</svg>"#
            ),
            Self::Play => format!(
                r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <polygon fill="{color}" points="4,3 13,8 4,13"/>
</svg>"#
            ),
            Self::Stop => format!(
                r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect fill="{color}" x="4" y="4" width="8" height="8"/>
</svg>"#
            ),
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
    icon.svg(color)
}

fn toolbar_button_message(icon: ToolbarIcon) -> GuiMessage {
    match icon {
        ToolbarIcon::Loop => GuiMessage::ToggleLoopPlayback,
        ToolbarIcon::Play => GuiMessage::PlaySelectedSample,
        ToolbarIcon::Stop => GuiMessage::StopPlayback,
    }
}
