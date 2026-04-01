//! In-process GUI test runner helpers.

mod assertions;
mod bundle;

use self::assertions::assert_scenario_state;
use self::bundle::snapshot_bundle;
use super::{
    GuiScenario, GuiScenarioStep, GuiStepTimingSample, GuiTestArtifactBundle, GuiTestModeConfig,
};
use crate::{
    app_core::actions::{GuiDispatchPolicy, NativeAppBridge, NativeUiAction, action_catalog_entry},
    gui_test::{GuiFixtureBridge, trace_event_for_action},
};
use std::{
    thread,
    time::{Duration, Instant},
};

/// Maximum time an in-process GUI scenario assertion waits for async state to settle.
const SCENARIO_ASSERT_TIMEOUT: Duration = Duration::from_millis(300);
/// Poll interval used while waiting for background-driven GUI state.
const SCENARIO_ASSERT_POLL_INTERVAL: Duration = Duration::from_millis(10);

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
    ensure_public_dispatch_action(&action)?;
    let mut bridge = make_bridge_for_fixture(&config.fixture_tag, config.viewport)?;
    let started = Instant::now();
    bridge.reduce_action(action.clone());
    let handled = bridge.take_last_action_handled().unwrap_or(true);
    let trace = vec![trace_event_for_action(&action, handled)];
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
            GuiScenarioStep::DispatchAction { action } => {
                ensure_public_dispatch_action(action)?;
                bridge.reduce_action(action.clone());
                let handled = bridge.take_last_action_handled().unwrap_or(true);
                trace.push(trace_event_for_action(action, handled));
            }
            GuiScenarioStep::Assert { assertion } => {
                let deadline = Instant::now() + SCENARIO_ASSERT_TIMEOUT;
                let failure_message = loop {
                    let snapshot = bundle::current_snapshot(config, &mut bridge);
                    match assert_scenario_state(&snapshot, &trace, assertion) {
                        Ok(()) => break None,
                        Err(err) if Instant::now() >= deadline => break Some(err),
                        Err(_) => thread::sleep(SCENARIO_ASSERT_POLL_INTERVAL),
                    }
                };
                if let Some(err) = failure_message {
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

fn ensure_public_dispatch_action(action: &NativeUiAction) -> Result<(), String> {
    let entry = action_catalog_entry(action);
    if entry.dispatch_policy == GuiDispatchPolicy::Public {
        return Ok(());
    }
    Err(format!(
        "action {} is cataloged for runtime-internal use and cannot be dispatched through the public GUI runner",
        entry.action_id
    ))
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn step_label(step: &GuiScenarioStep) -> String {
    match step {
        GuiScenarioStep::DispatchAction { action } => {
            format!("dispatch_action:{}", action_catalog_entry(action).action_id)
        }
        GuiScenarioStep::Assert { assertion } => format!("assert:{assertion:?}"),
    }
}

#[cfg(test)]
mod tests;
