//! Machine-readable GUI test artifacts shared by the CLI, bridge, and docs.

use crate::app_core::actions::{
    GUI_ACTION_CATALOG, GuiActionCatalogEntry, NativeAppModel, NativeGuiAutomationSnapshot,
    NativeUiAction, action_catalog_entry,
};
use serde::Serialize;
use std::{
    fs,
    io::ErrorKind,
    path::Path,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

const ARTIFACT_WRITE_RETRY_LIMIT: usize = 20;
const ARTIFACT_WRITE_RETRY_DELAY_MS: u64 = 25;

/// Serialized trace event for one GUI action observed during a test run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GuiActionTraceEvent {
    /// Stable action identifier resolved from the host catalog.
    pub action_id: String,
    /// Concrete serialized action payload.
    pub action: serde_json::Value,
    /// Whether the bridge accepted and handled the action.
    pub handled: bool,
    /// Unix-seconds timestamp for the event.
    pub observed_utc_secs: u64,
}

/// Compact projected-model summary stored beside automation snapshots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuiModelSummary {
    /// Main app title.
    pub title: String,
    /// Focus-context debug label.
    pub focus_context: String,
    /// Visible browser-row count.
    pub browser_visible_count: usize,
    /// Selected browser path count.
    pub browser_selected_count: usize,
    /// Projected browser viewport start row tracked by the controller.
    pub browser_view_start_row: usize,
    /// Whether browser focus currently requests guard-band autoscroll.
    pub browser_autoscroll: bool,
    /// Loaded waveform label, when present.
    pub waveform_loaded_label: Option<String>,
    /// Whether the confirm prompt is visible.
    pub prompt_visible: bool,
    /// Whether the options panel is visible.
    pub options_visible: bool,
    /// Whether the progress overlay is visible.
    pub progress_visible: bool,
    /// Update status debug label.
    pub update_status: String,
}

/// Timing sample for one named GUI scenario step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuiStepTimingSample {
    /// Step label.
    pub label: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

/// Full artifact bundle emitted by GUI test tools and runtime test mode.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GuiTestArtifactBundle {
    /// Schema version for forward-compatible readers.
    pub schema_version: u32,
    /// Scenario name associated with this bundle, when any.
    pub scenario_name: Option<String>,
    /// Fixture tag associated with this bundle.
    pub fixture_tag: String,
    /// Run-contract id associated with the bundle, when any.
    pub run_id: Option<String>,
    /// Run-contract manifest path associated with the bundle, when any.
    pub run_manifest_path: Option<String>,
    /// Latest semantic automation snapshot.
    pub automation_snapshot: NativeGuiAutomationSnapshot,
    /// Recorded action trace.
    pub action_trace: Vec<GuiActionTraceEvent>,
    /// Compact projected-model summary.
    pub model_summary: GuiModelSummary,
    /// Machine-readable action coverage report.
    pub action_catalog: Vec<GuiActionCatalogEntry>,
    /// Screenshot captured before failure, when available.
    pub screenshot_before_failure: Option<String>,
    /// Screenshot captured after failure, when available.
    pub screenshot_after_failure: Option<String>,
    /// Failure summary, when the run did not pass cleanly.
    pub failure_summary: Option<String>,
    /// Named timing samples recorded during the run.
    pub step_timings_ms: Vec<GuiStepTimingSample>,
}

/// Build a compact GUI-model summary from a projected native app model.
pub fn build_model_summary(model: &NativeAppModel) -> GuiModelSummary {
    GuiModelSummary {
        title: model.title.clone(),
        focus_context: format!("{:?}", model.focus_context),
        browser_visible_count: model.browser.visible_count,
        browser_selected_count: model.browser.selected_path_count,
        browser_view_start_row: model.browser.view_start_row,
        browser_autoscroll: model.browser.autoscroll,
        waveform_loaded_label: model.waveform.loaded_label.clone(),
        prompt_visible: model.confirm_prompt.visible,
        options_visible: model.options_panel.visible,
        progress_visible: model.progress_overlay.visible,
        update_status: format!("{:?}", model.update.status),
    }
}

/// Write one artifact bundle as pretty JSON, creating parent directories as needed.
pub fn write_artifact_bundle(bundle: &GuiTestArtifactBundle, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create artifact directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let json = serde_json::to_string_pretty(bundle)
        .map_err(|err| format!("failed to serialize GUI test bundle: {err}"))?;
    write_artifact_json_with_retry(path, json)
}

fn write_artifact_json_with_retry(path: &Path, json: String) -> Result<(), String> {
    for attempt in 0..ARTIFACT_WRITE_RETRY_LIMIT {
        match fs::write(path, &json) {
            Ok(()) => return Ok(()),
            Err(err)
                if attempt + 1 < ARTIFACT_WRITE_RETRY_LIMIT
                    && is_transient_artifact_write_error(&err) =>
            {
                thread::sleep(std::time::Duration::from_millis(
                    ARTIFACT_WRITE_RETRY_DELAY_MS,
                ));
            }
            Err(err) => {
                return Err(format!(
                    "failed to write GUI test bundle {}: {err}",
                    path.display()
                ));
            }
        }
    }
    Err(format!(
        "failed to write GUI test bundle {} after {} attempts",
        path.display(),
        ARTIFACT_WRITE_RETRY_LIMIT
    ))
}

fn is_transient_artifact_write_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        ErrorKind::PermissionDenied | ErrorKind::WouldBlock
    )
}

pub(crate) fn catalog_report() -> Vec<GuiActionCatalogEntry> {
    GUI_ACTION_CATALOG.to_vec()
}

pub(crate) fn trace_event_for_action(
    action: &NativeUiAction,
    handled: bool,
) -> GuiActionTraceEvent {
    GuiActionTraceEvent {
        action_id: String::from(action_catalog_entry(action).action_id),
        action: serde_json::to_value(action)
            .unwrap_or_else(|_| serde_json::json!({"error": "serialize"})),
        handled,
        observed_utc_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}
