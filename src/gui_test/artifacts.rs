//! Machine-readable GUI test artifacts shared by the CLI, bridge, and docs.

use crate::app_core::actions::{
    GUI_ACTION_CATALOG, GuiActionCatalogEntry, NativeAppModel, NativeAutomationNodeSnapshot,
    NativeAutomationRole, NativeGuiAutomationSnapshot, NativeUiAction, action_catalog_entry,
};
use radiant::gui::automation::{
    AutomationNodeSemantics, AutomationNodeSnapshot, AutomationRole, GuiAutomationSnapshot,
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

/// Runtime family that produced one GUI test artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GuiFixtureRuntime {
    /// Production `NativeAppState` lowered through the Radiant host boundary.
    NativeApp,
    /// Retained `AppController` compatibility fixture.
    LegacyController,
}

/// Observable worker ownership for one GUI fixture runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct GuiRuntimeComposition {
    /// Number of native filesystem watcher coordinators.
    pub native_source_watchers: usize,
    /// Number of native readiness supervisors.
    pub native_readiness_supervisors: usize,
    /// Number of retained controller analysis pools.
    pub legacy_analysis_pools: usize,
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
    /// Runtime run id associated with the bundle, when any.
    pub run_id: Option<String>,
    /// Runtime manifest path associated with the bundle, when any.
    pub run_manifest_path: Option<String>,
    /// Runtime family that produced the artifact.
    pub fixture_runtime: GuiFixtureRuntime,
    /// Observable background-worker ownership, when the runtime exposes it.
    pub runtime_composition: Option<GuiRuntimeComposition>,
    /// Artifact returned by the fixture runtime's shutdown hook.
    pub shutdown_artifact: Option<serde_json::Value>,
    /// Latest semantic automation snapshot.
    pub automation_snapshot: GuiAutomationSnapshot,
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

/// Build a compact summary from the production native automation tree.
pub(crate) fn build_native_model_summary(snapshot: &GuiAutomationSnapshot) -> GuiModelSummary {
    let mut nodes = Vec::new();
    collect_nodes(&snapshot.root, &mut nodes);
    let selected_rows = nodes
        .iter()
        .filter(|node| node.role == AutomationRole::Row && node.selected)
        .count();
    let visible_rows = nodes
        .iter()
        .filter(|node| node.role == AutomationRole::Row)
        .count();
    let waveform_loaded_label = nodes
        .iter()
        .find(|node| node.role == AutomationRole::TimelineRegion)
        .and_then(|node| node.label.clone().or_else(|| node.value.clone()));

    GuiModelSummary {
        title: String::from("Wavecrate"),
        focus_context: String::from("NativeApp"),
        browser_visible_count: visible_rows,
        browser_selected_count: selected_rows,
        browser_view_start_row: 0,
        browser_autoscroll: false,
        waveform_loaded_label,
        prompt_visible: nodes.iter().any(|node| node.role == AutomationRole::Dialog),
        options_visible: nodes.iter().any(|node| {
            node.role == AutomationRole::Panel
                && node
                    .label
                    .as_deref()
                    .is_some_and(|label| label.contains("Settings"))
        }),
        progress_visible: nodes.iter().any(|node| {
            node.role == AutomationRole::Readout
                && node
                    .label
                    .as_deref()
                    .is_some_and(|label| label.contains("progress"))
        }),
        update_status: String::from("Native"),
    }
}

fn collect_nodes<'a>(
    node: &'a AutomationNodeSnapshot,
    nodes: &mut Vec<&'a AutomationNodeSnapshot>,
) {
    nodes.push(node);
    for child in &node.children {
        collect_nodes(child, nodes);
    }
}

pub(crate) fn legacy_automation_snapshot_to_radiant(
    snapshot: NativeGuiAutomationSnapshot,
) -> GuiAutomationSnapshot {
    GuiAutomationSnapshot {
        schema_version: snapshot.schema_version,
        viewport_width: snapshot.viewport_width,
        viewport_height: snapshot.viewport_height,
        root: legacy_automation_node_to_radiant(snapshot.root),
    }
}

fn legacy_automation_node_to_radiant(node: NativeAutomationNodeSnapshot) -> AutomationNodeSnapshot {
    let semantics = AutomationNodeSemantics::new(match node.role {
        NativeAutomationRole::Root => AutomationRole::Root,
        NativeAutomationRole::Group => AutomationRole::Group,
        NativeAutomationRole::Panel => AutomationRole::Panel,
        NativeAutomationRole::Toolbar => AutomationRole::Toolbar,
        NativeAutomationRole::TabList => AutomationRole::TabList,
        NativeAutomationRole::Tab => AutomationRole::Tab,
        NativeAutomationRole::Button => AutomationRole::Button,
        NativeAutomationRole::SearchField => AutomationRole::SearchField,
        NativeAutomationRole::Slider => AutomationRole::Slider,
        NativeAutomationRole::Row => AutomationRole::Row,
        NativeAutomationRole::Table => AutomationRole::Table,
        NativeAutomationRole::WaveformRegion => AutomationRole::TimelineRegion,
        NativeAutomationRole::MapCanvas => AutomationRole::SpatialCanvas,
        NativeAutomationRole::MapPoint => AutomationRole::SpatialPoint,
        NativeAutomationRole::Readout => AutomationRole::Readout,
        NativeAutomationRole::Dialog => AutomationRole::Dialog,
    });
    let semantics = match node.label.as_ref() {
        Some(label) => semantics.with_label(label),
        None => semantics,
    };
    let mut semantics = match node.value.as_ref() {
        Some(value) => semantics.with_value_text(value),
        None => semantics,
    };
    semantics.disabled = !node.enabled;
    semantics.selected = node.selected;
    semantics.metadata = node.metadata.clone();
    let mut converted = AutomationNodeSnapshot::from_semantics(node.id, node.bounds, semantics);
    converted.enabled = node.enabled;
    converted.selected = node.selected;
    converted.available_actions = node.available_actions;
    converted.metadata = node.metadata;
    converted.children = node
        .children
        .into_iter()
        .map(legacy_automation_node_to_radiant)
        .collect();
    converted
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
