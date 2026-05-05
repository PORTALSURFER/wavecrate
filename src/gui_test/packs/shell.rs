//! Shell-chrome GUI contract scenarios outside the browser and waveform surfaces.

use crate::app_core::actions::NativeUiAction;
use crate::gui_test::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(super) fn options_open_close_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("options_open_close"),
        fixture_tag: String::from("browser"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeActionAvailable {
                    node_id: String::from("shell.top_bar.options_button"),
                    action_id: String::from("open_options_menu"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::OpenOptionsMenu,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("overlay.options_panel"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::CloseOptionsPanel,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeAbsent {
                    node_id: String::from("overlay.options_panel"),
                },
            },
        ],
    }
}

pub(super) fn update_panel_smoke_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("update_panel_smoke"),
        fixture_tag: String::from("update"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("shell.top_bar.update_panel"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("shell.top_bar.update_panel"),
                    key: String::from("status"),
                    needle: String::from("available"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("shell.top_bar.update.open"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeActionAvailable {
                    node_id: String::from("shell.top_bar.update.dismiss"),
                    action_id: String::from("dismiss_update"),
                },
            },
        ],
    }
}
