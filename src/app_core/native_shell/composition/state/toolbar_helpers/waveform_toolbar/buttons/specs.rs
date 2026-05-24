use super::*;

mod transport;

use transport::transport_button_spec;

pub(super) struct WaveformToolbarButtonSpec {
    pub(super) label: &'static str,
    pub(super) icon: Option<WaveformToolbarIcon>,
    pub(super) overlay_icon: Option<WaveformToolbarIcon>,
    pub(super) display_text: Option<String>,
    pub(super) enabled: bool,
    pub(super) active: bool,
    pub(super) action: Option<UiAction>,
    pub(super) text_color: Rgba8,
}

pub(super) fn waveform_toolbar_button_specs(
    style: &StyleTokens,
    model: &NativeMotionModel,
    bpm_input_active: bool,
    bpm_input_display: Option<&str>,
) -> Vec<WaveformToolbarButtonSpec> {
    let chrome = model.signal_chrome();
    let tools = model.signal_tools();
    let presentation = model.waveform_presentation();
    let raster_preview = model.waveform_image_preview();
    let loaded_available = raster_preview.loaded_label.is_some() && !raster_preview.loading;
    let bpm_value_label = waveform_toolbar_bpm_value_label(model, bpm_input_display);

    vec![
        WaveformToolbarButtonSpec {
            label: "Channel",
            icon: Some(channel_view_icon(chrome.channel_view)),
            overlay_icon: None,
            display_text: None,
            enabled: true,
            active: false,
            action: Some(UiAction::SetWaveformChannelView {
                stereo: chrome.channel_view != crate::gui::visualization::ChannelViewMode::Stereo,
            }),
            text_color: style.text_primary,
        },
        toggle_spec(
            "Norm",
            WaveformToolbarIcon::Normalize,
            tools.audition_enabled,
            Some(UiAction::SetNormalizedAuditionEnabled {
                enabled: !tools.audition_enabled,
            }),
            style,
        ),
        WaveformToolbarButtonSpec {
            label: "BPM Value",
            icon: None,
            overlay_icon: None,
            display_text: Some(bpm_value_label),
            enabled: true,
            active: bpm_input_active,
            action: None,
            text_color: style.text_primary,
        },
        toggle_spec(
            "BPM Snap",
            WaveformToolbarIcon::BpmSnap,
            tools.primary_snap_enabled,
            Some(UiAction::SetBpmSnapEnabled {
                enabled: !tools.primary_snap_enabled,
            }),
            style,
        ),
        toggle_spec(
            "Rel Grid",
            WaveformToolbarIcon::RelativeBpmGrid,
            tools.relative_grid_enabled,
            Some(UiAction::SetRelativeBpmGridEnabled {
                enabled: !tools.relative_grid_enabled,
            }),
            style,
        ),
        toggle_spec(
            "Tr Snap",
            WaveformToolbarIcon::TransientSnap,
            tools.secondary_snap_enabled,
            Some(UiAction::SetTransientSnapEnabled {
                enabled: !tools.secondary_snap_enabled,
            }),
            style,
        ),
        toggle_spec(
            "Show Tr",
            WaveformToolbarIcon::ShowTransients,
            tools.markers_visible,
            Some(UiAction::SetTransientMarkersEnabled {
                enabled: !tools.markers_visible,
            }),
            style,
        ),
        toggle_spec(
            "Slice",
            WaveformToolbarIcon::Slice,
            tools.review_mode_enabled,
            Some(UiAction::SetSliceModeEnabled {
                enabled: !tools.review_mode_enabled,
            }),
            style,
        ),
        command_spec(
            "Silence Split",
            loaded_available,
            Some(UiAction::DetectWaveformSilenceSlices),
            style.highlight_blue_soft,
        ),
        command_spec(
            "Exact Dedupe",
            loaded_available,
            Some(UiAction::DetectWaveformExactDuplicateSlices),
            style.highlight_blue_soft,
        ),
        command_spec(
            "Clean Dups",
            loaded_available && tools.cleanup_available,
            Some(UiAction::CleanWaveformExactDuplicateSlices),
            style.highlight_cyan_soft,
        ),
        loop_spec(style, tools.lock_enabled, presentation.repeat_enabled),
        compare_spec(style, chrome.reference_anchor_available),
        transport_button_spec(style, model.transport_running),
        WaveformToolbarButtonSpec {
            label: "Rec",
            icon: Some(WaveformToolbarIcon::Record),
            overlay_icon: None,
            display_text: None,
            enabled: false,
            active: false,
            action: None,
            text_color: style.highlight_blue_soft,
        },
    ]
}

fn channel_view_icon(
    channel_view: crate::gui::visualization::ChannelViewMode,
) -> WaveformToolbarIcon {
    if channel_view == crate::gui::visualization::ChannelViewMode::Stereo {
        WaveformToolbarIcon::Stereo
    } else {
        WaveformToolbarIcon::Mono
    }
}

fn toggle_spec(
    label: &'static str,
    icon: WaveformToolbarIcon,
    active: bool,
    action: Option<UiAction>,
    style: &StyleTokens,
) -> WaveformToolbarButtonSpec {
    WaveformToolbarButtonSpec {
        label,
        icon: Some(icon),
        overlay_icon: None,
        display_text: None,
        enabled: true,
        active,
        action,
        text_color: if active {
            style.accent_warning
        } else {
            style.text_muted
        },
    }
}

fn command_spec(
    label: &'static str,
    enabled: bool,
    action: Option<UiAction>,
    text_color: Rgba8,
) -> WaveformToolbarButtonSpec {
    WaveformToolbarButtonSpec {
        label,
        icon: Some(WaveformToolbarIcon::Slice),
        overlay_icon: None,
        display_text: None,
        enabled,
        active: false,
        action,
        text_color,
    }
}

fn loop_spec(
    style: &StyleTokens,
    lock_enabled: bool,
    repeat_enabled: bool,
) -> WaveformToolbarButtonSpec {
    WaveformToolbarButtonSpec {
        label: "Loop",
        icon: Some(WaveformToolbarIcon::Loop),
        overlay_icon: lock_enabled.then_some(WaveformToolbarIcon::Lock),
        display_text: None,
        enabled: true,
        active: repeat_enabled,
        action: Some(UiAction::ToggleLoopPlayback),
        text_color: if repeat_enabled {
            style.accent_warning
        } else {
            style.text_muted
        },
    }
}

fn compare_spec(style: &StyleTokens, available: bool) -> WaveformToolbarButtonSpec {
    WaveformToolbarButtonSpec {
        label: "Compare",
        icon: Some(WaveformToolbarIcon::Play),
        overlay_icon: None,
        display_text: None,
        enabled: available,
        active: false,
        action: Some(UiAction::PlayCompareAnchor),
        text_color: if available {
            style.highlight_cyan_soft
        } else {
            style.text_muted
        },
    }
}

impl WaveformToolbarButtonSpec {
    pub(super) fn into_button(self, rect: Rect) -> WaveformToolbarButton {
        WaveformToolbarButton {
            rect,
            label: self.label,
            icon: self.icon,
            overlay_icon: self.overlay_icon,
            display_text: self.display_text,
            enabled: self.enabled,
            active: self.active,
            action: self.action,
            text_color: self.text_color,
        }
    }
}
