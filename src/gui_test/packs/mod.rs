//! Reusable GUI scenario packs for contract and regression loops.

use super::GuiScenario;

mod browser;
mod map;
mod prompts;
mod shell;
mod transport;
mod waveform;

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
            browser::browser_search_and_commit_scenario(),
            transport::transport_play_from_selection_start_scenario(),
            transport::transport_volume_slider_scenario(),
            waveform::waveform_seek_zoom_selection_scenario(),
            map::map_point_focus_scenario(),
            shell::options_open_close_scenario(),
            prompts::prompt_confirm_scenario(),
            prompts::prompt_cancel_scenario(),
            shell::update_panel_smoke_scenario(),
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
