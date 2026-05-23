use super::assertions::assert_scenario_state;
use super::*;
use crate::app_core::actions::NativeAutomationBounds as AutomationBounds;
use crate::app_core::actions::{
    GUI_ACTION_CATALOG, NativeAutomationNodeId, NativeAutomationNodeSnapshot, NativeAutomationRole,
    NativeGuiAutomationSnapshot,
};
use crate::gui_test::{GuiActionTraceEvent, GuiAssertion, GuiScenario, GuiScenarioStep};
use std::collections::BTreeMap;

mod action_parity;

fn deterministic_test_config(fixture_tag: &str) -> GuiTestModeConfig {
    GuiTestModeConfig {
        fixture_tag: String::from(fixture_tag),
        ..GuiTestModeConfig::default()
    }
}

fn snapshot_with_child() -> NativeGuiAutomationSnapshot {
    NativeGuiAutomationSnapshot {
        schema_version: 1,
        viewport_width: 800,
        viewport_height: 600,
        root: NativeAutomationNodeSnapshot {
            id: NativeAutomationNodeId::new("shell.root"),
            role: NativeAutomationRole::Root,
            label: Some(String::from("Root")),
            bounds: AutomationBounds {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
            value: None,
            enabled: true,
            selected: false,
            available_actions: Vec::new(),
            metadata: BTreeMap::new(),
            children: vec![NativeAutomationNodeSnapshot {
                id: NativeAutomationNodeId::new("browser.search"),
                role: NativeAutomationRole::SearchField,
                label: Some(String::from("Search")),
                bounds: AutomationBounds {
                    x: 20.0,
                    y: 30.0,
                    width: 200.0,
                    height: 30.0,
                },
                value: Some(String::from("kick search")),
                enabled: true,
                selected: true,
                available_actions: vec![String::from("browser.search.commit")],
                metadata: BTreeMap::from([(
                    String::from("placeholder"),
                    String::from("Search samples"),
                )]),
                children: Vec::new(),
            }],
        },
    }
}

fn trace_with_action(action_id: &str, handled: bool) -> Vec<GuiActionTraceEvent> {
    vec![GuiActionTraceEvent {
        action_id: String::from(action_id),
        action: serde_json::json!({"kind": action_id}),
        handled,
        observed_utc_secs: 1,
    }]
}

fn collect_advertised_actions<'a>(
    node: &'a NativeAutomationNodeSnapshot,
    actions: &mut Vec<(&'a str, &'a str)>,
) {
    for action_id in &node.available_actions {
        actions.push((node.id.0.as_str(), action_id.as_str()));
    }
    for child in &node.children {
        collect_advertised_actions(child, actions);
    }
}

#[test]
fn capture_default_bundle_exposes_root_snapshot_and_catalog() {
    let bundle = capture_default_bundle(&deterministic_test_config("browser"))
        .expect("capture should succeed");
    assert_eq!(bundle.automation_snapshot.root.id.0, "shell.root");
    assert_eq!(bundle.action_catalog.len(), GUI_ACTION_CATALOG.len());
}

#[test]
fn scenario_runner_accepts_root_presence_assertion() {
    let scenario = GuiScenario {
        name: String::from("root-smoke"),
        fixture_tag: String::from("browser"),
        steps: vec![GuiScenarioStep::Assert {
            assertion: GuiAssertion::NodePresent {
                node_id: String::from("shell.root"),
            },
        }],
    };
    let bundle = run_scenario(&deterministic_test_config("browser"), &scenario)
        .expect("scenario should pass");
    assert!(bundle.failure_summary.is_none());
}

#[test]
fn dispatch_action_bundle_rejects_runtime_internal_actions() {
    let err = dispatch_action_bundle(
        &deterministic_test_config("waveform"),
        crate::app_core::actions::NativeUiAction::BeginWaveformSelectionShift {
            pointer_micros: 200_000,
            start_micros: 100_000,
            end_micros: 300_000,
        },
    )
    .expect_err("internal dispatch should be rejected");
    assert!(err.contains("begin_waveform_selection_shift"));
    assert!(err.contains("runtime-internal"));
}

#[test]
fn scenario_runner_rejects_runtime_internal_actions() {
    let scenario = GuiScenario {
        name: String::from("reject-runtime-internal"),
        fixture_tag: String::from("waveform"),
        steps: vec![GuiScenarioStep::DispatchAction {
            action: crate::app_core::actions::NativeUiAction::BeginWaveformEditSelectionShift {
                pointer_micros: 200_000,
                start_micros: 100_000,
                end_micros: 300_000,
            },
        }],
    };
    let err = run_scenario(&deterministic_test_config("waveform"), &scenario)
        .expect_err("internal dispatch should be rejected");
    assert!(err.contains("begin_waveform_edit_selection_shift"));
    assert!(err.contains("runtime-internal"));
}

#[test]
fn assert_scenario_state_accepts_each_assertion_variant() {
    let snapshot = snapshot_with_child();
    let catalog_action = GUI_ACTION_CATALOG[0].action_id;
    let trace = trace_with_action("browser.search.commit", true);
    let assertions = vec![
        GuiAssertion::NodePresent {
            node_id: String::from("browser.search"),
        },
        GuiAssertion::NodeAbsent {
            node_id: String::from("missing.node"),
        },
        GuiAssertion::NodeSelected {
            node_id: String::from("browser.search"),
            selected: true,
        },
        GuiAssertion::NodeEnabled {
            node_id: String::from("browser.search"),
            enabled: true,
        },
        GuiAssertion::NodeValueContains {
            node_id: String::from("browser.search"),
            needle: String::from("kick"),
        },
        GuiAssertion::NodeActionAvailable {
            node_id: String::from("browser.search"),
            action_id: String::from("browser.search.commit"),
        },
        GuiAssertion::NodeMetadataContains {
            node_id: String::from("browser.search"),
            key: String::from("placeholder"),
            needle: String::from("samples"),
        },
        GuiAssertion::NodeMetadataEquals {
            node_id: String::from("browser.search"),
            key: String::from("placeholder"),
            value: String::from("Search samples"),
        },
        GuiAssertion::ActionCataloged {
            action_id: String::from(catalog_action),
        },
        GuiAssertion::ActionRecorded {
            action_id: String::from("browser.search.commit"),
        },
    ];

    for assertion in assertions {
        assert!(
            assert_scenario_state(&snapshot, &trace, &assertion).is_ok(),
            "expected assertion to pass: {assertion:?}"
        );
    }
}

#[test]
fn assert_scenario_state_reports_missing_node_for_node_based_assertions() {
    let snapshot = snapshot_with_child();
    let trace = trace_with_action("browser.search.commit", true);
    let assertions = vec![
        GuiAssertion::NodePresent {
            node_id: String::from("missing.node"),
        },
        GuiAssertion::NodeSelected {
            node_id: String::from("missing.node"),
            selected: true,
        },
        GuiAssertion::NodeEnabled {
            node_id: String::from("missing.node"),
            enabled: true,
        },
        GuiAssertion::NodeValueContains {
            node_id: String::from("missing.node"),
            needle: String::from("kick"),
        },
        GuiAssertion::NodeActionAvailable {
            node_id: String::from("missing.node"),
            action_id: String::from("browser.search.commit"),
        },
        GuiAssertion::NodeMetadataContains {
            node_id: String::from("missing.node"),
            key: String::from("placeholder"),
            needle: String::from("samples"),
        },
    ];

    for assertion in assertions {
        let err = assert_scenario_state(&snapshot, &trace, &assertion).unwrap_err();
        assert!(
            err.contains("missing automation node missing.node"),
            "expected missing-node error, got: {err}"
        );
    }
}

#[test]
fn assert_scenario_state_reports_trace_and_catalog_failures() {
    let snapshot = snapshot_with_child();
    let trace = trace_with_action("browser.search.commit", true);

    let recorded_err = assert_scenario_state(
        &snapshot,
        &trace,
        &GuiAssertion::ActionRecorded {
            action_id: String::from("missing.action"),
        },
    )
    .unwrap_err();
    assert!(recorded_err.contains("handled action missing.action"));

    let catalog_err = assert_scenario_state(
        &snapshot,
        &trace,
        &GuiAssertion::ActionCataloged {
            action_id: String::from("missing.action"),
        },
    )
    .unwrap_err();
    assert!(catalog_err.contains("missing catalog action missing.action"));
}

#[test]
fn action_recorded_assertion_rejects_unhandled_trace_events() {
    let snapshot = snapshot_with_child();
    let trace = trace_with_action("browser.search.commit", false);

    let err = assert_scenario_state(
        &snapshot,
        &trace,
        &GuiAssertion::ActionRecorded {
            action_id: String::from("browser.search.commit"),
        },
    )
    .unwrap_err();
    assert!(err.contains("handled action browser.search.commit"));
}
