//! Browser-focused GUI contract scenarios.

use crate::app_core::actions::NativeUiAction;
use crate::gui_test::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(super) fn browser_search_and_commit_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("browser_search_select_commit"),
        fixture_tag: String::from("browser"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("browser.search_field"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::SetBrowserSearch {
                    query: String::from("snare"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("browser.search_field"),
                    needle: String::from("snare"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::FocusBrowserRow { visible_row: 0 },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.row.0"),
                    selected: true,
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::CommitFocusedBrowserRow,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.row.0"),
                    selected: true,
                },
            },
        ],
    }
}
