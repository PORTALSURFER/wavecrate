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
                action: NativeUiAction::ZoomWaveform {
                    zoom_in: true,
                    steps: 2,
                    anchor_ratio_micros: Some(200_000),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::ZoomWaveform {
                    zoom_in: true,
                    steps: 2,
                    anchor_ratio_micros: Some(800_000),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::SetWaveformViewCenter {
                    center_micros: 500_000,
                    center_nanos: Some(500_000_050),
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
                action: NativeUiAction::SetWaveformCursor {
                    position_milli: 500,
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeActionAvailable {
                    node_id: String::from("waveform.region"),
                    action_id: String::from("set_waveform_cursor"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::ZoomWaveformFull,
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
