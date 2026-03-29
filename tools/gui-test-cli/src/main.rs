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
    match parse_command(std::env::args().skip(1).collect())? {
        CliCommand::Snapshot { output } => {
            let config = cli_config(None, &output);
            let bundle = capture_default_bundle(&config)?;
            write_artifact_bundle(&bundle, &output)
        }
        CliCommand::DispatchAction {
            action_json,
            output,
        } => {
            let action: NativeUiAction = serde_json::from_str(&action_json)
                .map_err(|err| format!("failed to parse action JSON: {err}"))?;
            let config = cli_config(Some(String::from("dispatch-action")), &output);
            let bundle = dispatch_action_bundle(&config, action)?;
            write_artifact_bundle(&bundle, &output)
        }
        CliCommand::RunScenario {
            scenario_path,
            output,
        } => {
            let scenario_json = std::fs::read_to_string(&scenario_path).map_err(|err| {
                format!("failed to read scenario {}: {err}", scenario_path.display())
            })?;
            let scenario: GuiScenario = serde_json::from_str(&scenario_json)
                .map_err(|err| format!("failed to parse scenario JSON: {err}"))?;
            let config = cli_config(Some(scenario.name.clone()), &output);
            let bundle = run_scenario(&config, &scenario)?;
            write_artifact_bundle(&bundle, &output)
        }
        CliCommand::RunScenarioPack {
            pack_name,
            output_dir,
        } => {
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
        CliCommand::ExportAivSuite { args } => {
            let request = resolve_export_aiv_suite_request(args)?;
            match request.pack_name.as_deref() {
                Some(pack_name) => export_aiv_suite_pack(pack_name, &request.output_path),
                None => export_aiv_suite(&request.output_path),
            }
        }
        CliCommand::ResolveNodeTarget { artifact, node_id } => {
            let snapshot = read_automation_snapshot_from_artifact(&artifact)?;
            let target = resolve_automation_target(&snapshot, &node_id)?;
            let json = serde_json::to_string_pretty(&target)
                .map_err(|err| format!("failed to serialize automation target: {err}"))?;
            println!("{json}");
            Ok(())
        }
    }
}

#[derive(Debug)]
enum CliCommand {
    Snapshot { output: PathBuf },
    DispatchAction { action_json: String, output: PathBuf },
    RunScenario { scenario_path: PathBuf, output: PathBuf },
    RunScenarioPack { pack_name: String, output_dir: PathBuf },
    ExportAivSuite { args: Vec<String> },
    ResolveNodeTarget { artifact: PathBuf, node_id: String },
}

fn parse_command(args: Vec<String>) -> Result<CliCommand, String> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Err(usage());
    };
    match command.as_str() {
        "snapshot" => Ok(CliCommand::Snapshot {
            output: required_path(args.next(), "snapshot output path")?,
        }),
        "dispatch-action" => Ok(CliCommand::DispatchAction {
            action_json: required_arg(args.next(), "dispatch-action JSON payload")?,
            output: required_path(args.next(), "dispatch-action output path")?,
        }),
        "run-scenario" => Ok(CliCommand::RunScenario {
            scenario_path: required_path(args.next(), "scenario JSON path")?,
            output: required_path(args.next(), "scenario output path")?,
        }),
        "run-scenario-pack" => Ok(CliCommand::RunScenarioPack {
            pack_name: required_arg(args.next(), "scenario pack name")?,
            output_dir: required_path(args.next(), "scenario pack output dir")?,
        }),
        "export-aiv-suite" => Ok(CliCommand::ExportAivSuite { args: args.collect() }),
        "resolve-node-target" => Ok(CliCommand::ResolveNodeTarget {
            artifact: required_path(args.next(), "GUI artifact path")?,
            node_id: required_arg(args.next(), "automation node id")?,
        }),
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
    fn parse_snapshot_command_requires_output_path() {
        let command = parse_command(vec![String::from("snapshot"), String::from("out.json")])
            .expect("snapshot command");
        match command {
            CliCommand::Snapshot { output } => assert_eq!(output, PathBuf::from("out.json")),
            _ => panic!("expected snapshot command"),
        }
    }

    #[test]
    fn parse_dispatch_action_command_accepts_json_and_output() {
        let command = parse_command(vec![
            String::from("dispatch-action"),
            String::from(r#"{"kind":"PlayPause"}"#),
            String::from("artifact.json"),
        ])
        .expect("dispatch-action command");
        match command {
            CliCommand::DispatchAction {
                action_json,
                output,
            } => {
                assert_eq!(action_json, r#"{"kind":"PlayPause"}"#);
                assert_eq!(output, PathBuf::from("artifact.json"));
            }
            _ => panic!("expected dispatch-action command"),
        }
    }

    #[test]
    fn parse_run_scenario_command_accepts_paths() {
        let command = parse_command(vec![
            String::from("run-scenario"),
            String::from("scenario.json"),
            String::from("artifact.json"),
        ])
        .expect("run-scenario command");
        match command {
            CliCommand::RunScenario {
                scenario_path,
                output,
            } => {
                assert_eq!(scenario_path, PathBuf::from("scenario.json"));
                assert_eq!(output, PathBuf::from("artifact.json"));
            }
            _ => panic!("expected run-scenario command"),
        }
    }

    #[test]
    fn parse_run_scenario_pack_command_accepts_pack_and_output_dir() {
        let command = parse_command(vec![
            String::from("run-scenario-pack"),
            String::from("smoke"),
            String::from("artifacts"),
        ])
        .expect("run-scenario-pack command");
        match command {
            CliCommand::RunScenarioPack {
                pack_name,
                output_dir,
            } => {
                assert_eq!(pack_name, "smoke");
                assert_eq!(output_dir, PathBuf::from("artifacts"));
            }
            _ => panic!("expected run-scenario-pack command"),
        }
    }

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

    #[test]
    fn parse_export_aiv_suite_command_preserves_remaining_args() {
        let command = parse_command(vec![
            String::from("export-aiv-suite"),
            String::from("desktop-regression"),
            String::from("out.json"),
        ])
        .expect("export-aiv-suite command");
        match command {
            CliCommand::ExportAivSuite { args } => assert_eq!(
                args,
                vec![
                    String::from("desktop-regression"),
                    String::from("out.json")
                ]
            ),
            _ => panic!("expected export-aiv-suite command"),
        }
    }

    #[test]
    fn parse_resolve_node_target_command_accepts_artifact_and_node_id() {
        let command = parse_command(vec![
            String::from("resolve-node-target"),
            String::from("artifact.json"),
            String::from("browser.root"),
        ])
        .expect("resolve-node-target command");
        match command {
            CliCommand::ResolveNodeTarget { artifact, node_id } => {
                assert_eq!(artifact, PathBuf::from("artifact.json"));
                assert_eq!(node_id, "browser.root");
            }
            _ => panic!("expected resolve-node-target command"),
        }
    }

    #[test]
    fn parse_unknown_command_reports_usage() {
        let err = parse_command(vec![String::from("unknown")]).expect_err("unknown command");
        assert!(err.contains("unknown command 'unknown'"));
        assert!(err.contains("gui-test-cli snapshot <output.json>"));
    }
}
