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

pub(super) fn browser_focus_transition_stability_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("browser_focus_transition_stability"),
        fixture_tag: String::from("browser"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("browser.panel"),
                    key: String::from("focused_sample_label"),
                    needle: String::from("kick_one"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("waveform.region"),
                    needle: String::from("kick_one"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::FocusBrowserRow { visible_row: 1 },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::MoveBrowserFocus { delta: 1 },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::ActionRecorded {
                    action_id: String::from("move_browser_focus"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.row.2"),
                    selected: true,
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("browser.panel"),
                    key: String::from("focused_sample_label"),
                    needle: String::from("hat_three"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("waveform.region"),
                    needle: String::from("kick_one"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::CommitFocusedBrowserRow,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::ActionRecorded {
                    action_id: String::from("commit_focused_browser_row"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataContains {
                    node_id: String::from("browser.panel"),
                    key: String::from("focused_sample_label"),
                    needle: String::from("hat_three"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeValueContains {
                    node_id: String::from("waveform.region"),
                    needle: String::from("kick_one"),
                },
            },
        ],
    }
}

pub(super) fn browser_tag_sidebar_unified_tag_library_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("browser_tag_sidebar_unified_tag_library"),
        fixture_tag: String::from("browser"),
        steps: vec![
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::ToggleBrowserTagSidebar,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("browser.tag_sidebar"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeAbsent {
                    node_id: String::from("browser.tag_sidebar.custom_tag"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataEquals {
                    node_id: String::from("browser.tag_sidebar"),
                    key: String::from("normal_tag_labels"),
                    value: String::from("Texture|Deep Kick|Rare FX"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.tag_sidebar.normal_tag.texture"),
                    selected: true,
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::SetBrowserTagSidebarInput {
                    value: String::from("rfx"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeMetadataEquals {
                    node_id: String::from("browser.tag_sidebar"),
                    key: String::from("normal_tag_labels"),
                    value: String::from("Rare FX"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.tag_sidebar.normal_tag.rare_fx"),
                    selected: false,
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::ToggleBrowserSidebarNormalTag {
                    label: String::from("Rare FX"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.tag_sidebar.normal_tag.rare_fx"),
                    selected: true,
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::SetBrowserTagSidebarInput {
                    value: String::from("vinyl crackle"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("browser.tag_sidebar.create_tag.vinyl_crackle"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::CommitBrowserTagSidebarInput,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.tag_sidebar.normal_tag.vinyl_crackle"),
                    selected: true,
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::FocusBrowserRow { visible_row: 1 },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.tag_sidebar.normal_tag.vinyl_crackle"),
                    selected: false,
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::ToggleBrowserSidebarNormalTag {
                    label: String::from("Vinyl Crackle"),
                },
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeSelected {
                    node_id: String::from("browser.tag_sidebar.normal_tag.vinyl_crackle"),
                    selected: true,
                },
            },
        ],
    }
}
