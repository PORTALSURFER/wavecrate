use super::*;

pub(super) fn transport_button_spec(
    style: &StyleTokens,
    transport_running: bool,
) -> WaveformToolbarButtonSpec {
    if transport_running {
        return WaveformToolbarButtonSpec {
            label: "Stop",
            icon: Some(WaveformToolbarIcon::Stop),
            overlay_icon: None,
            display_text: None,
            enabled: true,
            active: true,
            action: Some(UiAction::HandleEscape),
            text_color: style.highlight_orange_soft,
        };
    }

    WaveformToolbarButtonSpec {
        label: "Play",
        icon: Some(WaveformToolbarIcon::Play),
        overlay_icon: None,
        display_text: None,
        enabled: true,
        active: false,
        action: Some(UiAction::ToggleTransport),
        text_color: style.accent_warning,
    }
}
