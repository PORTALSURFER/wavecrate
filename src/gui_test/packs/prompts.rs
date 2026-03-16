//! Prompt-focused GUI contract scenarios.

use crate::app_core::actions::NativeUiAction;
use crate::gui_test::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(super) fn prompt_confirm_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("prompt_confirm"),
        fixture_tag: String::from("prompt"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("overlay.prompt.confirm"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::SetPromptInput {
                    value: String::from("kick_smoke.wav"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::ConfirmPrompt,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeAbsent {
                    node_id: String::from("overlay.prompt.confirm"),
                },
            },
        ],
    }
}

pub(super) fn prompt_cancel_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("prompt_cancel"),
        fixture_tag: String::from("prompt"),
        steps: vec![
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("overlay.prompt.confirm"),
                },
            },
            GuiScenarioStep::DispatchAction {
                action: NativeUiAction::CancelPrompt,
            },
            GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodeAbsent {
                    node_id: String::from("overlay.prompt.confirm"),
                },
            },
        ],
    }
}
