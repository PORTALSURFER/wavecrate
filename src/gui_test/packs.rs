//! Reusable GUI scenario packs for contract and regression loops.

use super::{GuiAssertion, GuiScenario, GuiScenarioStep};
use crate::app_core::actions::NativeUiAction;

/// Named collection of deterministic GUI scenarios.
#[derive(Clone, Debug, PartialEq)]
pub struct GuiScenarioPack {
    /// Stable pack identifier.
    pub name: &'static str,
    /// Ordered scenarios executed for the pack.
    pub scenarios: Vec<GuiScenario>,
}

/// Resolve one named GUI scenario pack.
pub fn gui_scenario_pack(name: &str) -> Result<GuiScenarioPack, String> {
    match name {
        "contract-smoke" => Ok(contract_smoke_pack()),
        other => Err(format!("unknown GUI scenario pack '{other}'")),
    }
}

fn contract_smoke_pack() -> GuiScenarioPack {
    GuiScenarioPack {
        name: "contract-smoke",
        scenarios: vec![
            browser_search_and_commit_scenario(),
            waveform_seek_zoom_selection_scenario(),
            options_open_close_scenario(),
            prompt_confirm_scenario(),
            prompt_cancel_scenario(),
            update_panel_smoke_scenario(),
        ],
    }
}

fn browser_search_and_commit_scenario() -> GuiScenario {
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

fn waveform_seek_zoom_selection_scenario() -> GuiScenario {
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

fn options_open_close_scenario() -> GuiScenario {
    GuiScenario {
        name: String::from("options_open_close"),
        fixture_tag: String::from("default"),
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

fn prompt_confirm_scenario() -> GuiScenario {
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

fn prompt_cancel_scenario() -> GuiScenario {
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

fn update_panel_smoke_scenario() -> GuiScenario {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_test::{GuiTestModeConfig, run_scenario};

    #[test]
    fn contract_smoke_pack_runs_cleanly() {
        let pack = gui_scenario_pack("contract-smoke").expect("pack");
        for scenario in &pack.scenarios {
            let bundle =
                run_scenario(&GuiTestModeConfig::default(), scenario).unwrap_or_else(|err| {
                    panic!("scenario {} failed to execute: {err}", scenario.name)
                });
            assert!(
                bundle.failure_summary.is_none(),
                "scenario {} failed: {:?}",
                scenario.name,
                bundle.failure_summary
            );
        }
    }
}
