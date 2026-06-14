//! Waveform-focused GUI contract scenarios.

use crate::app_core::actions::NativeUiAction;
use crate::gui_test::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(super) fn waveform_seek_zoom_selection_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("waveform_seek_zoom_selection"),
        fixture_tag: String::from("waveform"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("waveform.region"),
                    needle: String::from("kick"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.region"),
                    key: String::from("zoom_label"),
                    needle: String::from("200%"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.selection"),
                    key: String::from("selection_micros"),
                    needle: String::from("350000-450000"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull,
                ),
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                        start_micros: 180_000,
                        end_micros: 420_000,
                        snap_override: true,
                        preserve_view_edge: false,
                    },
                ),
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.selection"),
                    key: String::from("selection_micros"),
                    needle: String::from("180000-420000"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                        zoom_in: true,
                        steps: 2,
                        anchor_ratio_micros: Some(200_000),
                    },
                ),
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                        zoom_in: true,
                        steps: 2,
                        anchor_ratio_micros: Some(800_000),
                    },
                ),
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                        zoom_in: true,
                        steps: 2,
                        anchor_ratio_micros: Some(500_000),
                    },
                ),
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
                        center_micros: 500_000,
                        center_nanos: Some(500_000_050),
                    },
                ),
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.selection"),
                    key: String::from("selection_micros"),
                    needle: String::from("180000-420000"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                        start_micros: 500_180,
                        end_micros: 500_420,
                        snap_override: true,
                        preserve_view_edge: false,
                    },
                ),
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.selection"),
                    key: String::from("selection_micros"),
                    needle: String::from("500180-500420"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                        zoom_in: true,
                        steps: 3,
                        anchor_ratio_micros: Some(100_000),
                    },
                ),
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                        zoom_in: false,
                        steps: 1,
                        anchor_ratio_micros: Some(900_000),
                    },
                ),
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
                        center_micros: 500_300,
                        center_nanos: Some(500_300_000),
                    },
                ),
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.selection"),
                    key: String::from("selection_micros"),
                    needle: String::from("500180-500420"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
                        position_nanos: 500_000_000,
                    },
                ),
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeActionAvailable {
                    node_id: String::from("waveform.region"),
                    action_id: String::from("set_waveform_cursor_precise"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::Waveform(
                    crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull,
                ),
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.region"),
                    key: String::from("zoom_label"),
                    needle: String::from("100%"),
                },
            },
        ],
    }
}
