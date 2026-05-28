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
            browser::browser_focus_transition_stability_scenario(),
            browser::browser_tag_sidebar_unified_tag_library_scenario(),
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
    use crate::gui_test::{GuiTestModeConfig, run_scenario, run_scenario_batch};

    fn assert_bundle_passed(bundle: &crate::gui_test::GuiTestArtifactBundle) {
        let scenario_name = bundle.scenario_name.as_deref().unwrap_or("<unknown>");
        assert!(
            bundle.failure_summary.is_none(),
            "scenario {} failed: {:?}",
            scenario_name,
            bundle.failure_summary
        );
    }

    fn scenario_by_name<'a>(pack: &'a GuiScenarioPack, name: &str) -> &'a GuiScenario {
        pack.scenarios
            .iter()
            .find(|scenario| scenario.name == name)
            .unwrap_or_else(|| panic!("missing scenario {name}"))
    }

    #[test]
    #[ignore = "runs through scripts/gui.ps1 contract; too expensive for the default lib test lane"]
    fn contract_smoke_pack_runs_cleanly() {
        let pack = gui_scenario_pack("contract-smoke").expect("pack");
        let config = GuiTestModeConfig::default();
        let browser_search_options_batch = [
            scenario_by_name(&pack, "browser_search_select_commit").clone(),
            scenario_by_name(&pack, "options_open_close").clone(),
        ];
        let transport_batch = [
            scenario_by_name(&pack, "transport_play_from_selection_start").clone(),
            scenario_by_name(&pack, "transport_volume_slider").clone(),
        ];

        for batch in [&browser_search_options_batch[..], &transport_batch[..]] {
            for bundle in run_scenario_batch(&config, batch)
                .unwrap_or_else(|err| panic!("scenario batch failed to execute: {err}"))
            {
                assert_bundle_passed(&bundle);
            }
        }

        for scenario_name in [
            "browser_focus_transition_stability",
            "left_sidebar_unified_tag_library",
            "waveform_seek_zoom_selection",
            "map_point_focus",
            "prompt_confirm",
            "prompt_cancel",
            "update_panel_smoke",
        ] {
            let scenario = scenario_by_name(&pack, scenario_name);
            let bundle = run_scenario(&config, scenario).unwrap_or_else(|err| {
                panic!("scenario {} failed to execute: {err}", scenario.name)
            });
            assert_bundle_passed(&bundle);
        }
    }
}
