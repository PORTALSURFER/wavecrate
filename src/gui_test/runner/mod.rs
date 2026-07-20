//! In-process GUI test runner helpers.

mod assertions;
mod bundle;

use self::assertions::{assert_scenario_state, assert_trace_or_catalog_state};
use self::bundle::snapshot_bundle;
use super::{
    GuiScenario, GuiScenarioStep, GuiStepTimingSample, GuiTestArtifactBundle, GuiTestModeConfig,
};
use crate::{
    app_core::actions::{GuiDispatchPolicy, NativeUiAction, action_catalog_entry},
    app_dirs::{ConfigBaseGuard, PersistenceProfileGuard},
    gui_test::{
        GuiFixtureBridge, GuiFixtureRuntime, GuiRuntimeComposition, build_native_model_summary,
        gui_test_fixture_uses_isolated_startup, gui_test_fixture_uses_live_profile,
        trace_event_for_action,
    },
    sample_sources::{
        SampleSource,
        config::{AppConfig, AppSettingsCore},
    },
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
    if fixture_uses_native_app(&config.fixture_tag) {
        return capture_native_startup_bundle(config);
    }
    let mut bridge = make_bridge_for_fixture(&config.fixture_tag, config.viewport)?;
    Ok(snapshot_bundle(
        config,
        &mut bridge,
        Vec::new(),
        None,
        Vec::new(),
    ))
}

/// Dispatch one UI action through the default bridge fixture and capture a bundle.
pub fn dispatch_action_bundle(
    config: &GuiTestModeConfig,
    action: NativeUiAction,
) -> Result<GuiTestArtifactBundle, String> {
    ensure_public_dispatch_action(&action)?;
    ensure_legacy_fixture_for_controller_actions(&config.fixture_tag)?;
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
    ensure_legacy_fixture_for_controller_actions(&scenario.fixture_tag)?;
    let mut bridge = make_bridge_for_fixture(&scenario.fixture_tag, config.viewport)?;
    run_scenario_with_bridge(config, scenario, &mut bridge)
}

fn fixture_uses_native_app(fixture_tag: &str) -> bool {
    gui_test_fixture_uses_live_profile(fixture_tag)
        || gui_test_fixture_uses_isolated_startup(fixture_tag)
}

fn ensure_legacy_fixture_for_controller_actions(fixture_tag: &str) -> Result<(), String> {
    if !fixture_uses_native_app(fixture_tag) {
        return Ok(());
    }
    Err(format!(
        "fixture {fixture_tag} is product-native and does not accept retained controller actions; use a named legacy fixture for NativeUiAction scenarios"
    ))
}

fn capture_native_startup_bundle(
    config: &GuiTestModeConfig,
) -> Result<GuiTestArtifactBundle, String> {
    if gui_test_fixture_uses_live_profile(&config.fixture_tag) {
        let _profile_guard = PersistenceProfileGuard::live();
        return native_bundle_from_capture(
            config,
            crate::native_app::automation::capture_startup(config.viewport)?,
        );
    }

    let config_base = tempfile::tempdir()
        .map_err(|err| format!("create isolated native GUI config base: {err}"))?;
    let source_root = tempfile::tempdir()
        .map_err(|err| format!("create isolated native GUI source root: {err}"))?;
    let _base_guard = ConfigBaseGuard::set(config_base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::automated();
    crate::sample_sources::config::save(&AppConfig {
        sources: vec![SampleSource::new(source_root.path().to_path_buf())],
        core: AppSettingsCore::default(),
    })
    .map_err(|err| format!("seed isolated native GUI profile: {err}"))?;
    native_bundle_from_capture(
        config,
        crate::native_app::automation::capture_startup(config.viewport)?,
    )
}

fn native_bundle_from_capture(
    config: &GuiTestModeConfig,
    capture: crate::native_app::automation::NativeAutomationCapture,
) -> Result<GuiTestArtifactBundle, String> {
    let model_summary = build_native_model_summary(&capture.automation_snapshot);
    Ok(GuiTestArtifactBundle {
        schema_version: 2,
        scenario_name: config.scenario_name.clone(),
        fixture_tag: config.fixture_tag.clone(),
        run_id: config.run_id.clone(),
        run_manifest_path: config
            .run_manifest_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        fixture_runtime: GuiFixtureRuntime::NativeApp,
        runtime_composition: Some(GuiRuntimeComposition {
            native_source_watchers: capture.runtime_composition.source_watcher_count,
            native_readiness_supervisors: capture.runtime_composition.readiness_supervisor_count,
            legacy_analysis_pools: capture.runtime_composition.legacy_analysis_pool_count,
        }),
        shutdown_artifact: capture.shutdown_artifact,
        automation_snapshot: capture.automation_snapshot,
        action_trace: Vec::new(),
        model_summary,
        action_catalog: Vec::new(),
        screenshot_before_failure: None,
        screenshot_after_failure: None,
        failure_summary: None,
        step_timings_ms: Vec::new(),
    })
}

/// Run a sequence of scenarios against one shared fixture bridge.
///
/// Batch runs are intended for contract suites where setup dominates runtime and
/// the scenarios form one deterministic journey from the same fixture state.
#[cfg(test)]
pub(crate) fn run_scenario_batch(
    config: &GuiTestModeConfig,
    scenarios: &[GuiScenario],
) -> Result<Vec<GuiTestArtifactBundle>, String> {
    let Some(first) = scenarios.first() else {
        return Ok(Vec::new());
    };
    if let Some(mismatched) = scenarios
        .iter()
        .find(|scenario| scenario.fixture_tag != first.fixture_tag)
    {
        return Err(format!(
            "scenario batch mixes fixture {} with {}",
            first.fixture_tag, mismatched.fixture_tag
        ));
    }
    let mut bridge = make_bridge_for_fixture(&first.fixture_tag, config.viewport)?;
    scenarios
        .iter()
        .map(|scenario| run_scenario_with_bridge(config, scenario, &mut bridge))
        .collect()
}

fn run_scenario_with_bridge(
    config: &GuiTestModeConfig,
    scenario: &GuiScenario,
    bridge: &mut GuiFixtureBridge,
) -> Result<GuiTestArtifactBundle, String> {
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
                let failure_message =
                    if let Some(result) = assert_trace_or_catalog_state(&trace, assertion) {
                        result.err()
                    } else {
                        let deadline = Instant::now() + SCENARIO_ASSERT_TIMEOUT;
                        loop {
                            let snapshot = bundle::current_snapshot(config, bridge);
                            match assert_scenario_state(&snapshot, &trace, assertion) {
                                Ok(()) => break None,
                                Err(err) if Instant::now() >= deadline => break Some(err),
                                Err(_) => thread::sleep(SCENARIO_ASSERT_POLL_INTERVAL),
                            }
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
    let mut bundle = snapshot_bundle(config, bridge, trace, failure, timings);
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
