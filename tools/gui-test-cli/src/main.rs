#![deny(missing_docs)]
#![deny(warnings)]

//! Command-line entrypoint for deterministic GUI test artifacts and scenario runs.

use sempal::{
    app_core::actions::NativeUiAction,
    gui_test::{
        GuiScenario, GuiTestModeConfig, capture_default_bundle, dispatch_action_bundle,
        export_aiv_suite, export_aiv_suite_pack, gui_scenario_pack,
        read_automation_snapshot_from_artifact, resolve_automation_target, run_scenario,
        write_artifact_bundle,
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
        "run-scenario-pack" => {
            let pack_name = required_arg(args.next(), "scenario pack name")?;
            let output_dir = required_path(args.next(), "scenario pack output dir")?;
            let pack = gui_scenario_pack(&pack_name)?;
            for scenario in &pack.scenarios {
                let output = output_dir.join(format!("{}.json", scenario.name));
                let mut config = cli_config(Some(scenario.name.clone()), &output);
                config.fixture_tag = scenario.fixture_tag.clone();
                let bundle = run_scenario(&config, scenario)?;
                write_artifact_bundle(&bundle, &output)?;
            }
            Ok(())
        }
        "export-aiv-suite" => {
            let request = resolve_export_aiv_suite_request(args.collect())?;
            match request.pack_name.as_deref() {
                Some(pack_name) => export_aiv_suite_pack(pack_name, &request.output_path),
                None => export_aiv_suite(&request.output_path),
            }
        }
        "resolve-node-target" => {
            let artifact = required_path(args.next(), "GUI artifact path")?;
            let node_id = required_arg(args.next(), "automation node id")?;
            let snapshot = read_automation_snapshot_from_artifact(&artifact)?;
            let target = resolve_automation_target(&snapshot, &node_id)?;
            let json = serde_json::to_string_pretty(&target)
                .map_err(|err| format!("failed to serialize automation target: {err}"))?;
            println!("{json}");
            Ok(())
        }
        other => Err(format!("unknown command '{other}'\n\n{}", usage())),
    }
}

fn cli_config(scenario_name: Option<String>, output_path: &std::path::Path) -> GuiTestModeConfig {
    let mut config = GuiTestModeConfig::from_env(None, None).unwrap_or_default();
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
        "usage:\n  gui-test-cli snapshot <output.json>\n  gui-test-cli dispatch-action <action-json> <output.json>\n  gui-test-cli run-scenario <scenario.json> <output.json>\n  gui-test-cli run-scenario-pack <pack-name> <output-dir>\n  gui-test-cli export-aiv-suite <output.json>\n  gui-test-cli export-aiv-suite <pack-name> <output.json>\n  gui-test-cli resolve-node-target <artifact.json> <node-id>",
    )
}

struct ExportAivSuiteRequest {
    pack_name: Option<String>,
    output_path: PathBuf,
}

fn resolve_export_aiv_suite_request(args: Vec<String>) -> Result<ExportAivSuiteRequest, String> {
    match args.as_slice() {
        [output_path] => Ok(ExportAivSuiteRequest {
            pack_name: None,
            output_path: PathBuf::from(output_path),
        }),
        [pack_name, output_path] => Ok(ExportAivSuiteRequest {
            pack_name: Some(pack_name.clone()),
            output_path: PathBuf::from(output_path),
        }),
        _ => Err(format!("missing export-aiv-suite arguments\n\n{}", usage())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_aiv_suite_legacy_alias_defaults_to_smoke_pack() {
        let request = resolve_export_aiv_suite_request(vec![String::from("out.json")])
            .expect("legacy export request");
        assert_eq!(request.pack_name, None);
        assert_eq!(request.output_path, PathBuf::from("out.json"));
    }

    #[test]
    fn export_aiv_suite_accepts_explicit_pack_name() {
        let request = resolve_export_aiv_suite_request(vec![
            String::from("desktop-regression"),
            String::from("out.json"),
        ])
        .expect("explicit export request");
        assert_eq!(request.pack_name.as_deref(), Some("desktop-regression"));
        assert_eq!(request.output_path, PathBuf::from("out.json"));
    }
}
