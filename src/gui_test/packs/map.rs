//! Map-focused GUI contract scenarios.

use crate::app_core::actions::NativeUiAction;
use crate::gui_test::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(super) fn map_point_focus_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("map_point_focus"),
        fixture_tag: String::from("map"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.tab.map"),
                    selected: true,
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("browser.map_canvas"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("browser.map.point.gui-map-source::kick_one.wav"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::FocusMapSample {
                    sample_id: String::from("gui-map-source::kick_one.wav"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::ActionRecorded {
                    action_id: String::from("focus_map_sample"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.map.point.gui-map-source::kick_one.wav"),
                    selected: true,
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("waveform.region"),
                    needle: String::from("kick"),
                },
            },
        ],
    }
}
