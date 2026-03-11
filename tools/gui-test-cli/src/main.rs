#![deny(missing_docs)]
#![deny(warnings)]

//! Command-line entrypoint for deterministic GUI test artifacts and scenario runs.

use sempal::{
    app_core::actions::NativeUiAction,
    gui_test::{
        GuiScenario, GuiTestModeConfig, capture_default_bundle, dispatch_action_bundle,
        export_aiv_suite, run_scenario, write_artifact_bundle,
    },
};
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(usage());
    };
    match command.as_str() {
        "snapshot" => {
            let output = required_path(args.next(), "snapshot output path")?;
            let config = cli_config(None, &output);
            let bundle = capture_default_bundle(&config)?;
            write_artifact_bundle(&bundle, &output)
        }
        "dispatch-action" => {
            let action_json = required_arg(args.next(), "dispatch-action JSON payload")?;
            let output = required_path(args.next(), "dispatch-action output path")?;
            let action: NativeUiAction = serde_json::from_str(&action_json)
                .map_err(|err| format!("failed to parse action JSON: {err}"))?;
            let config = cli_config(Some(String::from("dispatch-action")), &output);
            let bundle = dispatch_action_bundle(&config, action)?;
            write_artifact_bundle(&bundle, &output)
        }
        "run-scenario" => {
            let scenario_path = required_path(args.next(), "scenario JSON path")?;
            let output = required_path(args.next(), "scenario output path")?;
            let scenario_json = std::fs::read_to_string(&scenario_path).map_err(|err| {
                format!("failed to read scenario {}: {err}", scenario_path.display())
            })?;
            let scenario: GuiScenario = serde_json::from_str(&scenario_json)
                .map_err(|err| format!("failed to parse scenario JSON: {err}"))?;
            let config = cli_config(Some(scenario.name.clone()), &output);
            let bundle = run_scenario(&config, &scenario)?;
            write_artifact_bundle(&bundle, &output)
        }
        "export-aiv-suite" => {
            let output = required_path(args.next(), "AIV suite output path")?;
            let config = cli_config(Some(String::from("aiv-suite")), &output);
            export_aiv_suite(&config, &output)
        }
        other => Err(format!("unknown command '{other}'\n\n{}", usage())),
    }
}

fn cli_config(scenario_name: Option<String>, output_path: &std::path::Path) -> GuiTestModeConfig {
    let mut config = GuiTestModeConfig::default();
    config.scenario_name = scenario_name;
    config.artifact_dir = output_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    config
}

fn required_arg(value: Option<String>, what: &str) -> Result<String, String> {
    value.ok_or_else(|| format!("missing {what}\n\n{}", usage()))
}

fn required_path(value: Option<String>, what: &str) -> Result<PathBuf, String> {
    required_arg(value, what).map(PathBuf::from)
}

fn usage() -> String {
    String::from(
        "usage:\n  gui-test-cli snapshot <output.json>\n  gui-test-cli dispatch-action <action-json> <output.json>\n  gui-test-cli run-scenario <scenario.json> <output.json>\n  gui-test-cli export-aiv-suite <output.json>",
    )
}
