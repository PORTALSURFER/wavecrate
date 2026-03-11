//! Live GUI test artifact emission wired into the native bridge.

use crate::{
    app_core::actions::NativeAppModel,
    gui_runtime::capture_gui_automation_snapshot,
    gui_test::{
        GuiTestArtifactBundle, GuiTestModeConfig, build_model_summary, catalog_report,
        trace_event_for_action, write_artifact_bundle,
    },
};
use crate::app_core::actions::NativeUiAction;
use std::path::PathBuf;
use tracing::warn;

/// Bridge-owned GUI test recorder that mirrors live runtime state into artifacts.
pub(super) struct BridgeGuiTestRecorder {
    config: GuiTestModeConfig,
    action_trace: Vec<crate::gui_test::GuiActionTraceEvent>,
    projected_once: bool,
}

impl BridgeGuiTestRecorder {
    /// Create a new bridge recorder for one GUI test-mode session.
    pub(super) fn new(config: GuiTestModeConfig) -> Self {
        Self {
            config,
            action_trace: Vec::new(),
            projected_once: false,
        }
    }

    /// Record the first projected model snapshot for startup readiness consumers.
    pub(super) fn record_projected_model(&mut self, model: &NativeAppModel) {
        if self.projected_once {
            return;
        }
        self.projected_once = true;
        self.write_bundle(model, None);
    }

    /// Record one reduced action and the resulting projected model snapshot.
    pub(super) fn record_action(&mut self, action: &NativeUiAction, model: &NativeAppModel) {
        self.action_trace.push(trace_event_for_action(action));
        self.write_bundle(model, None);
    }

    fn write_bundle(&self, model: &NativeAppModel, failure_summary: Option<String>) {
        let bundle = GuiTestArtifactBundle {
            schema_version: 1,
            scenario_name: self.config.scenario_name.clone(),
            fixture_tag: self.config.fixture_tag.clone(),
            run_id: self.config.run_id.clone(),
            run_manifest_path: self
                .config
                .run_manifest_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            automation_snapshot: capture_gui_automation_snapshot(self.config.viewport_f32(), model),
            action_trace: self.action_trace.clone(),
            model_summary: build_model_summary(model),
            action_catalog: catalog_report(),
            screenshot_before_failure: None,
            screenshot_after_failure: None,
            failure_summary,
            step_timings_ms: Vec::new(),
        };
        if let Err(err) = write_artifact_bundle(&bundle, &self.bundle_path()) {
            warn!(path = %self.bundle_path().display(), %err, "failed to write live GUI test artifact");
        }
    }

    fn bundle_path(&self) -> PathBuf {
        self.config.artifact_dir.join("gui_test_latest.json")
    }
}
