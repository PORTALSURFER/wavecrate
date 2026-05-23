use super::*;
use crate::app_core::app_api::state::{
    BrowserBitDepthFacet, BrowserBpmFacet, BrowserChannelFacet, BrowserFormatFacet,
    BrowserKeyFacet, BrowserSidebarFilterFacet, BrowserSidebarFilterOption,
};

/// Build the selectable option buttons for one sidebar filter dropdown.
pub(super) fn sidebar_filter_dropdown_buttons(
    facet: SidebarFilterDropdownFacet,
    model: &AppModel,
    style: &StyleTokens,
) -> Vec<ActionButton> {
    match facet {
        SidebarFilterDropdownFacet::Format => format_filter_buttons(model, style),
        SidebarFilterDropdownFacet::BitDepth => bit_depth_filter_buttons(model, style),
        SidebarFilterDropdownFacet::Channels => channel_filter_buttons(model, style),
        SidebarFilterDropdownFacet::Bpm => bpm_filter_buttons(model, style),
        SidebarFilterDropdownFacet::Key => key_filter_buttons(model, style),
        SidebarFilterDropdownFacet::Rating => rating_filter_buttons(model, style),
    }
}

fn format_filter_buttons(model: &AppModel, style: &StyleTokens) -> Vec<ActionButton> {
    vec![
        sidebar_toggle_button(
            "WAV",
            model
                .sidebar_filters
                .formats
                .contains(&BrowserFormatFacet::Wav),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Format(BrowserFormatFacet::Wav),
                additive: true,
            },
            style,
        ),
        sidebar_clear_button(
            BrowserSidebarFilterFacet::Format,
            !model.sidebar_filters.formats.is_empty(),
            style,
        ),
    ]
}

fn bit_depth_filter_buttons(model: &AppModel, style: &StyleTokens) -> Vec<ActionButton> {
    vec![
        sidebar_toggle_button(
            "Unavailable",
            model
                .sidebar_filters
                .bit_depths
                .contains(&BrowserBitDepthFacet::Unavailable),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::BitDepth(BrowserBitDepthFacet::Unavailable),
                additive: true,
            },
            style,
        ),
        sidebar_clear_button(
            BrowserSidebarFilterFacet::BitDepth,
            !model.sidebar_filters.bit_depths.is_empty(),
            style,
        ),
    ]
}

fn key_filter_buttons(model: &AppModel, style: &StyleTokens) -> Vec<ActionButton> {
    vec![
        sidebar_toggle_button(
            "Unknown",
            model
                .sidebar_filters
                .keys
                .contains(&BrowserKeyFacet::Unknown),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Key(BrowserKeyFacet::Unknown),
                additive: true,
            },
            style,
        ),
        sidebar_clear_button(
            BrowserSidebarFilterFacet::Key,
            !model.sidebar_filters.keys.is_empty(),
            style,
        ),
    ]
}

fn channel_filter_buttons(model: &AppModel, style: &StyleTokens) -> Vec<ActionButton> {
    vec![
        sidebar_toggle_button(
            "Mono",
            model
                .sidebar_filters
                .channels
                .contains(&BrowserChannelFacet::Mono),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Channels(BrowserChannelFacet::Mono),
                additive: true,
            },
            style,
        ),
        sidebar_toggle_button(
            "Stereo",
            model
                .sidebar_filters
                .channels
                .contains(&BrowserChannelFacet::Stereo),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Channels(BrowserChannelFacet::Stereo),
                additive: true,
            },
            style,
        ),
        sidebar_toggle_button(
            "Multi",
            model
                .sidebar_filters
                .channels
                .contains(&BrowserChannelFacet::Multi),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Channels(BrowserChannelFacet::Multi),
                additive: true,
            },
            style,
        ),
        sidebar_toggle_button(
            "Unavailable",
            model
                .sidebar_filters
                .channels
                .contains(&BrowserChannelFacet::Unavailable),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Channels(BrowserChannelFacet::Unavailable),
                additive: true,
            },
            style,
        ),
        sidebar_clear_button(
            BrowserSidebarFilterFacet::Channels,
            !model.sidebar_filters.channels.is_empty(),
            style,
        ),
    ]
}

fn bpm_filter_buttons(model: &AppModel, style: &StyleTokens) -> Vec<ActionButton> {
    vec![
        sidebar_toggle_button(
            "Unknown",
            model
                .sidebar_filters
                .bpms
                .contains(&BrowserBpmFacet::Unknown),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Bpm(BrowserBpmFacet::Unknown),
                additive: true,
            },
            style,
        ),
        sidebar_toggle_button(
            "Slow <90",
            model.sidebar_filters.bpms.contains(&BrowserBpmFacet::Slow),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Bpm(BrowserBpmFacet::Slow),
                additive: true,
            },
            style,
        ),
        sidebar_toggle_button(
            "Mid 90-129",
            model.sidebar_filters.bpms.contains(&BrowserBpmFacet::Mid),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Bpm(BrowserBpmFacet::Mid),
                additive: true,
            },
            style,
        ),
        sidebar_toggle_button(
            "Fast 130+",
            model.sidebar_filters.bpms.contains(&BrowserBpmFacet::Fast),
            UiAction::ToggleBrowserSidebarFilter {
                option: BrowserSidebarFilterOption::Bpm(BrowserBpmFacet::Fast),
                additive: true,
            },
            style,
        ),
        sidebar_clear_button(
            BrowserSidebarFilterFacet::Bpm,
            !model.sidebar_filters.bpms.is_empty(),
            style,
        ),
    ]
}

fn rating_filter_buttons(model: &AppModel, style: &StyleTokens) -> Vec<ActionButton> {
    [-3, -2, -1, 0, 1, 2, 3, 4]
        .into_iter()
        .enumerate()
        .map(|(index, level)| {
            sidebar_toggle_button(
                rating_dropdown_label(level),
                model.browser.active_rating_filters[index],
                UiAction::ToggleBrowserRatingFilter {
                    level,
                    invert: false,
                },
                style,
            )
        })
        .collect()
}

/// Build a dropdown toggle button with active-state styling metadata.
fn sidebar_toggle_button(
    label: &'static str,
    active: bool,
    action: UiAction,
    style: &StyleTokens,
) -> ActionButton {
    ActionButton {
        rect: Rect::default(),
        label,
        icon: None,
        enabled: true,
        active,
        action,
        text_color: if active {
            style.accent_mint
        } else {
            style.text_primary
        },
    }
}

/// Build a dropdown clear button for one sidebar filter facet.
fn sidebar_clear_button(
    facet: BrowserSidebarFilterFacet,
    enabled: bool,
    style: &StyleTokens,
) -> ActionButton {
    ActionButton {
        rect: Rect::default(),
        label: "Clear",
        icon: None,
        enabled,
        active: false,
        action: UiAction::ClearBrowserSidebarFilter { facet },
        text_color: if enabled {
            style.accent_copper
        } else {
            style.text_muted
        },
    }
}

/// Return the user-facing rating dropdown label for one rating level.
fn rating_dropdown_label(level: i8) -> &'static str {
    match level {
        -3 => "-3",
        -2 => "-2",
        -1 => "-1",
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "Locked",
        _ => "",
    }
}
