//! In-process GUI test runner helpers.

use super::{
    GuiActionTraceEvent, GuiAssertion, GuiScenario, GuiScenarioStep, GuiStepTimingSample,
    GuiTestArtifactBundle, GuiTestModeConfig, build_model_summary, find_automation_node,
};
use crate::{
    app_core::actions::{
        GUI_ACTION_CATALOG, NativeAppBridge, NativeUiAction, action_catalog_entry,
        action_catalog_entry_by_id,
    },
    gui_runtime::capture_gui_automation_snapshot,
    gui_test::GuiFixtureBridge,
    gui_test::trace_event_for_action,
};
use std::time::Instant;

/// Capture a deterministic automation bundle from the default bridge fixture.
pub fn capture_default_bundle(config: &GuiTestModeConfig) -> Result<GuiTestArtifactBundle, String> {
    let mut bridge = make_bridge_for_fixture(&config.fixture_tag, config.viewport)?;
    Ok(snapshot_bundle(
        config,
        &mut bridge,
        Vec::new(),
        None,
        Vec::new(),
    ))
}

/// Dispatch one native action through the default bridge fixture and capture a bundle.
pub fn dispatch_action_bundle(
    config: &GuiTestModeConfig,
    action: NativeUiAction,
) -> Result<GuiTestArtifactBundle, String> {
    let mut bridge = make_bridge_for_fixture(&config.fixture_tag, config.viewport)?;
    let started = Instant::now();
    bridge.reduce_action(action.clone());
    let trace = vec![trace_event_for_action(&action)];
    let timings = vec![GuiStepTimingSample {
        label: String::from("dispatch_action"),
        duration_ms: elapsed_ms(started),
    }];
    Ok(snapshot_bundle(config, &mut bridge, trace, None, timings))
}

/// Run a declarative GUI scenario against the default bridge fixture.
pub fn run_scenario(
    config: &GuiTestModeConfig,
    scenario: &GuiScenario,
) -> Result<GuiTestArtifactBundle, String> {
    let mut bridge = make_bridge_for_fixture(&scenario.fixture_tag, config.viewport)?;
    let mut trace = Vec::new();
    let mut timings = Vec::new();
    let mut failure = None;
    for step in &scenario.steps {
        let started = Instant::now();
        match step {
            GuiScenarioStep::CaptureSnapshot { .. } => {}
            GuiScenarioStep::DispatchAction { action } => {
                bridge.reduce_action(action.clone());
                trace.push(trace_event_for_action(action));
            }
            GuiScenarioStep::Assert { assertion } => {
                let snapshot = current_snapshot(config, &mut bridge);
                if let Err(err) = assert_snapshot(&snapshot, assertion) {
                    failure = Some(err);
                    timings.push(GuiStepTimingSample {
                        label: step_label(step),
                        duration_ms: elapsed_ms(started),
                    });
                    break;
                }
            }
        }
        timings.push(GuiStepTimingSample {
            label: step_label(step),
            duration_ms: elapsed_ms(started),
        });
    }
    let mut bundle = snapshot_bundle(config, &mut bridge, trace, failure, timings);
    bundle.scenario_name = Some(scenario.name.clone());
    bundle.fixture_tag = scenario.fixture_tag.clone();
    Ok(bundle)
}

fn make_bridge_for_fixture(
    fixture_tag: &str,
    viewport: [u32; 2],
) -> Result<GuiFixtureBridge, String> {
    GuiFixtureBridge::new_with_viewport(fixture_tag, viewport)
}

fn snapshot_bundle(
    config: &GuiTestModeConfig,
    bridge: &mut impl NativeAppBridge,
    trace: Vec<GuiActionTraceEvent>,
    failure_summary: Option<String>,
    step_timings_ms: Vec<GuiStepTimingSample>,
) -> GuiTestArtifactBundle {
    let model = bridge.project_model();
    GuiTestArtifactBundle {
        schema_version: 1,
        scenario_name: config.scenario_name.clone(),
        fixture_tag: config.fixture_tag.clone(),
        run_id: config.run_id.clone(),
        run_manifest_path: config
            .run_manifest_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        automation_snapshot: capture_gui_automation_snapshot(config.viewport_f32(), model.as_ref()),
        action_trace: trace,
        model_summary: build_model_summary(model.as_ref()),
        action_catalog: GUI_ACTION_CATALOG.to_vec(),
        screenshot_before_failure: None,
        screenshot_after_failure: None,
        failure_summary,
        step_timings_ms,
    }
}

fn current_snapshot(
    config: &GuiTestModeConfig,
    bridge: &mut impl NativeAppBridge,
) -> crate::app_core::actions::NativeGuiAutomationSnapshot {
    let model = bridge.project_model();
    capture_gui_automation_snapshot(config.viewport_f32(), model.as_ref())
}

fn assert_snapshot(
    snapshot: &crate::app_core::actions::NativeGuiAutomationSnapshot,
    assertion: &GuiAssertion,
) -> Result<(), String> {
    match assertion {
        GuiAssertion::NodePresent { node_id } => find_automation_node(snapshot, node_id)
            .map(|_| ())
            .ok_or_else(|| format!("missing automation node {node_id}")),
        GuiAssertion::NodeAbsent { node_id } => find_automation_node(snapshot, node_id)
            .is_none()
            .then_some(())
            .ok_or_else(|| format!("unexpected automation node {node_id}")),
        GuiAssertion::NodeSelected { node_id, selected } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            if node.selected == *selected {
                Ok(())
            } else {
                Err(format!(
                    "automation node {node_id} selected={} expected={selected}",
                    node.selected
                ))
            }
        }
        GuiAssertion::NodeEnabled { node_id, enabled } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            if node.enabled == *enabled {
                Ok(())
            } else {
                Err(format!(
                    "automation node {node_id} enabled={} expected={enabled}",
                    node.enabled
                ))
            }
        }
        GuiAssertion::NodeValueContains { node_id, needle } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            let value = node.value.as_deref().unwrap_or_default();
            value.contains(needle).then_some(()).ok_or_else(|| {
                format!("automation node {node_id} value '{value}' does not contain '{needle}'")
            })
        }
        GuiAssertion::NodeActionAvailable { node_id, action_id } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            node.available_actions
                .iter()
                .any(|available| available == action_id)
                .then_some(())
                .ok_or_else(|| {
                    format!("automation node {node_id} does not advertise action {action_id}")
                })
        }
        GuiAssertion::NodeMetadataContains {
            node_id,
            key,
            needle,
        } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            let actual = node
                .metadata
                .get(key)
                .map(String::as_str)
                .unwrap_or_default();
            actual
                .contains(needle)
                .then_some(())
                .ok_or_else(|| {
                    format!(
                        "automation node {node_id} metadata {key}='{actual}' does not contain '{needle}'"
                    )
                })
        }
        GuiAssertion::ActionCataloged { action_id } => action_catalog_entry_by_id(action_id)
            .map(|_| ())
            .ok_or_else(|| format!("missing catalog action {action_id}")),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn step_label(step: &GuiScenarioStep) -> String {
    match step {
        GuiScenarioStep::CaptureSnapshot { label } => format!("capture_snapshot:{label}"),
        GuiScenarioStep::DispatchAction { action } => {
            format!("dispatch_action:{}", action_catalog_entry(action).action_id)
        }
        GuiScenarioStep::Assert { assertion } => format!("assert:{assertion:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_test::{GuiAssertion, GuiScenario, GuiScenarioStep};

    #[test]
    fn capture_default_bundle_exposes_root_snapshot_and_catalog() {
        let bundle =
            capture_default_bundle(&GuiTestModeConfig::default()).expect("capture should succeed");
        assert_eq!(bundle.automation_snapshot.root.id.0, "shell.root");
        assert_eq!(bundle.action_catalog.len(), GUI_ACTION_CATALOG.len());
    }

    #[test]
    fn scenario_runner_accepts_root_presence_assertion() {
        let scenario = GuiScenario {
            name: String::from("root-smoke"),
            fixture_tag: String::from("default"),
            steps: vec![GuiScenarioStep::Assert {
                assertion: GuiAssertion::NodePresent {
                    node_id: String::from("shell.root"),
                },
            }],
        };
        let bundle =
            run_scenario(&GuiTestModeConfig::default(), &scenario).expect("scenario should pass");
        assert!(bundle.failure_summary.is_none());
    }
}
