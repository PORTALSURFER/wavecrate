//! Transport-focused GUI contract scenarios.

use crate::app_core::actions::NativeUiAction;
use crate::gui_test::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(super) fn transport_play_from_selection_start_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("transport_play_from_selection_start"),
        fixture_tag: String::from("transport"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("waveform.toolbar.play"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeActionAvailable {
                    node_id: String::from("waveform.toolbar.play"),
                    action_id: String::from("toggle_transport"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("waveform.region"),
                    key: String::from("cursor_milli"),
                    needle: String::from("380"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::PlayFromStart,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::ActionRecorded {
                    action_id: String::from("play_from_start"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::ActionCataloged {
                    action_id: String::from("play_from_start"),
                },
            },
        ],
    }
}

pub(super) fn transport_volume_slider_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("transport_volume_slider"),
        fixture_tag: String::from("transport"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("shell.top_bar.volume_slider"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeActionAvailable {
                    node_id: String::from("shell.top_bar.volume_slider"),
                    action_id: String::from("set_volume"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeActionAvailable {
                    node_id: String::from("shell.top_bar.volume_slider"),
                    action_id: String::from("commit_volume_setting"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("shell.top_bar.volume_slider"),
                    needle: String::from("0.420"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::SetVolume { value_milli: 750 },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::ActionRecorded {
                    action_id: String::from("set_volume"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("shell.top_bar.volume_slider"),
                    needle: String::from("0.750"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::ActionCataloged {
                    action_id: String::from("commit_volume_setting"),
                },
            },
        ],
    }
}
