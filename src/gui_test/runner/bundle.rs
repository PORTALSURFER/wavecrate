//! Bundle capture helpers for the in-process GUI scenario runner.

use crate::{
    app_core::actions::{GUI_ACTION_CATALOG, NativeAppBridge, NativeGuiAutomationSnapshot},
    gui_runtime::capture_gui_automation_snapshot,
    gui_test::{
        GuiActionTraceEvent, GuiStepTimingSample, GuiTestArtifactBundle, GuiTestModeConfig,
        build_model_summary,
    },
};

/// Build one artifact bundle from the current bridge projection and trace state.
pub(super) fn snapshot_bundle(
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

/// Capture the current semantic automation snapshot from the bridge.
pub(super) fn current_snapshot(
    config: &GuiTestModeConfig,
    bridge: &mut impl NativeAppBridge,
) -> NativeGuiAutomationSnapshot {
    let model = bridge.project_model();
    capture_gui_automation_snapshot(config.viewport_f32(), model.as_ref())
}
