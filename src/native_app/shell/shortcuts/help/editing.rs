use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::waveform::WaveformInteraction;

use super::{ShortcutHelpEntry, shortcut_binding, shortcut_help_entry, x_help_label};

pub(super) fn waveform_shortcut_help_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    let mut entries = vec![
        key_entry(
            "Waveform",
            "Enter",
            "Apply edit mark edits",
            ui::KeyCode::Enter,
            GuiMessage::RequestApplyEditSelectionEffects,
        ),
        key_entry(
            "Waveform",
            "E",
            "Extract play selection or selected files",
            ui::KeyCode::E,
            GuiMessage::ExtractPlaymarkedRange,
        ),
        key_entry(
            "Waveform",
            "W",
            "Open context menu",
            ui::KeyCode::W,
            GuiMessage::OpenContextMenu,
        ),
        shortcut_help_entry(
            "Waveform",
            "Command-E",
            "Extract and trim selection",
            [shortcut_binding(
                ui::KeyPress::with_command(ui::KeyCode::E),
                GuiMessage::RequestExtractAndTrimWaveformSelection,
            )],
        ),
        key_entry(
            "Waveform",
            "Z",
            "Zoom to play selection",
            ui::KeyCode::Z,
            GuiMessage::Waveform(WaveformInteraction::ZoomToPlaySelection),
        ),
    ];
    if super::super::playmark_slide_shortcut_active(state) {
        entries.push(playmark_slide_entry());
    }
    entries.extend([
        key_entry(
            "Waveform",
            "C",
            "Crop selection",
            ui::KeyCode::C,
            GuiMessage::RequestCropWaveformSelection,
        ),
        key_entry(
            "Waveform",
            "D",
            "Trim selection",
            ui::KeyCode::D,
            GuiMessage::RequestTrimWaveformSelection,
        ),
        key_entry(
            "Waveform",
            "R",
            "Reverse selection",
            ui::KeyCode::R,
            GuiMessage::RequestReverseWaveformSelection,
        ),
        key_entry(
            "Waveform",
            "M",
            "Mute selection",
            ui::KeyCode::M,
            GuiMessage::RequestMuteWaveformSelection,
        ),
    ]);
    if super::super::waveform_zoom_out_shortcut_active(state) {
        entries.push(key_entry(
            "Waveform",
            "X",
            x_help_label(state),
            ui::KeyCode::X,
            super::super::x_shortcut_action(state),
        ));
    }
    entries.push(key_entry(
        "Waveform",
        "L",
        "Toggle loop playback",
        ui::KeyCode::L,
        GuiMessage::ToggleLoopPlayback,
    ));
    entries
}

pub(super) fn navigation_shortcut_help_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    let mut entries = vec![
        navigation_entry("Up / Down", "Move browser selection", false, false),
        navigation_entry(
            "Shift-Up / Shift-Down",
            "Extend sample selection",
            true,
            false,
        ),
        navigation_entry(
            "Command-Up / Command-Down",
            "Move focus without changing marks",
            false,
            true,
        ),
    ];
    if !super::super::playmark_slide_shortcut_active(state) {
        entries.extend([
            key_entry(
                "Navigation",
                "Left",
                "Collapse selected folder",
                ui::KeyCode::ArrowLeft,
                super::super::left_arrow_shortcut_action(state),
            ),
            key_entry(
                "Navigation",
                "Right",
                "Play from current play start",
                ui::KeyCode::ArrowRight,
                super::super::right_arrow_shortcut_action(state),
            ),
        ]);
    }
    entries.push(key_entry(
        "Navigation",
        "F",
        "Focus selected map node",
        ui::KeyCode::F,
        GuiMessage::FocusSelectedStarmapNode,
    ));
    entries
}

fn playmark_slide_entry() -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Waveform",
        "Left / Right",
        "Slide play selection",
        [
            shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::ArrowLeft),
                GuiMessage::Waveform(WaveformInteraction::SlidePlaySelection { direction: -1 }),
            ),
            shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::ArrowRight),
                GuiMessage::Waveform(WaveformInteraction::SlidePlaySelection { direction: 1 }),
            ),
        ],
    )
}

fn navigation_entry(
    keys: &'static str,
    action: &'static str,
    extend: bool,
    preserve_selection: bool,
) -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Navigation",
        keys,
        action,
        [
            shortcut_binding(
                arrow_press(ui::KeyCode::ArrowUp, extend, preserve_selection),
                GuiMessage::NavigateBrowser {
                    delta: -1,
                    extend,
                    preserve_selection,
                },
            ),
            shortcut_binding(
                arrow_press(ui::KeyCode::ArrowDown, extend, preserve_selection),
                GuiMessage::NavigateBrowser {
                    delta: 1,
                    extend,
                    preserve_selection,
                },
            ),
        ],
    )
}

fn arrow_press(key: ui::KeyCode, shift: bool, command: bool) -> ui::KeyPress {
    ui::KeyPress {
        key,
        command,
        control: false,
        shift,
        alt: false,
    }
}

fn key_entry(
    section: &'static str,
    keys: &'static str,
    action: &'static str,
    key: ui::KeyCode,
    message: GuiMessage,
) -> ShortcutHelpEntry {
    shortcut_help_entry(
        section,
        keys,
        action,
        [shortcut_binding(ui::KeyPress::new(key), message)],
    )
}
