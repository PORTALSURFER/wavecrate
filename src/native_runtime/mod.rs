//! Wavecrate GUI runtime host integration shared by the current `native_app` shell.

use crate::app_core::actions::{
    NativeAppModel, NativeAutomationBounds, NativeAutomationNodeId, NativeAutomationNodeSnapshot,
    NativeAutomationRole, NativeGuiAutomationSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub use radiant::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, NativeStartupTimingArtifact, RuntimeRunReport,
};

/// Bundled Wavecrate UI font asset.
pub const WAVECRATE_UI_FONT_ASSET: &str = "assets/FORCED SQUARE.ttf";

/// Return the bundled Wavecrate UI font path used before native font fallbacks.
pub fn wavecrate_ui_font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WAVECRATE_UI_FONT_ASSET)
}

/// Machine-readable native shutdown timing payload exported by Wavecrate bridges.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NativeShutdownTimingArtifact {
    /// Whether all shutdown phases completed without a captured error.
    pub status: String,
    /// Explicit shutdown failure reason when a phase reports an error.
    pub failure_reason: Option<String>,
    /// Milliseconds spent flushing bridge-owned pending input before exit.
    pub bridge_exit_flush_ms: Option<f64>,
    /// Milliseconds spent persisting host configuration during exit.
    pub config_persist_ms: Option<f64>,
    /// Milliseconds spent draining controller job workers.
    pub controller_jobs_shutdown_ms: Option<f64>,
    /// Milliseconds spent draining analysis workers.
    pub analysis_shutdown_ms: Option<f64>,
    /// Milliseconds spent inside the controller shutdown boundary.
    pub controller_shutdown_ms: Option<f64>,
    /// Milliseconds spent inside the full runtime-exit hook.
    pub runtime_exit_total_ms: Option<f64>,
}

/// Structured runtime artifacts exported after one Wavecrate native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Wavecrate shutdown timing artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<NativeShutdownTimingArtifact>,
}

/// Result plus structured artifacts returned by one Wavecrate UI runtime execution.
#[derive(Debug)]
pub struct NativeRunReport {
    /// Structured artifacts captured during the run.
    pub artifacts: NativeRuntimeArtifacts,
    /// UI runtime success or error outcome.
    pub result: Result<(), String>,
}

/// RGBA icon payload used by UI runtime hosts.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// UI runtime launch options for Vello hosts.
#[derive(Clone, Debug, Default)]
pub struct NativeRunOptions {
    /// Window title.
    pub title: String,
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
    /// Whether the window starts maximized.
    pub maximized: bool,
    /// Whether native window decorations remain enabled.
    pub decorations: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
    /// Target frame rate for animation-driven redraws.
    pub target_fps: u32,
    /// Paint red layout-boundary strokes over every projected layout element.
    pub debug_layout: bool,
}

/// Capture a deterministic GUI automation snapshot without launching the native host.
pub fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    let viewport_width = viewport[0].max(0.0).round() as u32;
    let viewport_height = viewport[1].max(0.0).round() as u32;
    NativeGuiAutomationSnapshot {
        schema_version: 1,
        viewport_width,
        viewport_height,
        root: automation_node(
            "shell.root",
            NativeAutomationRole::Root,
            Some(format!("{} shell", model.title)),
            bounds(0.0, 0.0, viewport[0], viewport[1]),
            None,
            false,
            shell_children(viewport, model),
        ),
    }
}

fn shell_children(viewport: [f32; 2], model: &NativeAppModel) -> Vec<NativeAutomationNodeSnapshot> {
    let width = viewport[0].max(0.0);
    let height = viewport[1].max(0.0);
    let top_height = 40.0;
    let status_height = 28.0;
    let sidebar_width = 264.0_f32.min(width * 0.4);
    let content_width = (width - sidebar_width).max(0.0);
    let content_height = (height - top_height - status_height).max(0.0);
    let waveform_height = (content_height * 0.44).max(0.0);

    vec![
        automation_node(
            "shell.top_bar",
            NativeAutomationRole::Panel,
            Some(String::from("Top bar")),
            bounds(0.0, 0.0, width, top_height),
            None,
            false,
            Vec::new(),
        ),
        sources_panel(sidebar_width, top_height, content_height, model),
        automation_node(
            "waveform.panel",
            NativeAutomationRole::Panel,
            Some(String::from("Waveform")),
            bounds(sidebar_width, top_height, content_width, waveform_height),
            None,
            false,
            Vec::new(),
        ),
        browser_panel(
            sidebar_width,
            top_height + waveform_height,
            content_width,
            (content_height - waveform_height).max(0.0),
            model,
        ),
        status_bar(
            width,
            (height - status_height).max(0.0),
            status_height,
            model,
        ),
    ]
}

fn sources_panel(
    width: f32,
    y: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    automation_node(
        "sources.panel",
        NativeAutomationRole::Panel,
        Some(String::from("Sources")),
        bounds(0.0, y, width, height),
        None,
        false,
        vec![
            automation_node(
                "sources.source_list",
                NativeAutomationRole::Table,
                Some(String::from("Sources")),
                bounds(0.0, y, width, (height * 0.36).max(0.0)),
                Some(format!("{} sources", model.sources.rows.len())),
                false,
                Vec::new(),
            ),
            automation_node(
                "sources.folder_browser",
                NativeAutomationRole::Table,
                Some(String::from("Folders")),
                bounds(0.0, y + height * 0.36, width, (height * 0.64).max(0.0)),
                Some(format!(
                    "{} folders",
                    model.sources.upper_folder_pane.tree_rows.len()
                )),
                false,
                Vec::new(),
            ),
        ],
    )
}

fn browser_panel(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    automation_node(
        "browser.panel",
        NativeAutomationRole::Panel,
        Some(String::from("Browser")),
        bounds(x, y, width, height),
        None,
        false,
        vec![automation_node(
            "browser.table",
            NativeAutomationRole::Table,
            Some(String::from("Samples")),
            bounds(x, y, width, height),
            Some(format!("{} rows", model.browser.visible_count)),
            false,
            model
                .browser
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    automation_node(
                        format!("browser.row.{index}"),
                        NativeAutomationRole::Row,
                        Some(row.label.to_string()),
                        bounds(x, y + 24.0 + index as f32 * 22.0, width, 22.0),
                        None,
                        model.browser.selected_visible_row == Some(index),
                        Vec::new(),
                    )
                })
                .collect(),
        )],
    )
}

fn status_bar(
    width: f32,
    y: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    let mut metadata = BTreeMap::new();
    metadata.insert(String::from("left"), model.status.left.clone());
    metadata.insert(String::from("center"), model.status.center.clone());
    metadata.insert(String::from("right"), model.status.right.clone());
    let mut node = automation_node(
        "shell.status_bar",
        NativeAutomationRole::Readout,
        Some(String::from("Status bar")),
        bounds(0.0, y, width, height),
        Some(model.status.center.clone()),
        false,
        Vec::new(),
    );
    node.metadata = metadata;
    node
}

fn automation_node(
    id: impl Into<String>,
    role: NativeAutomationRole,
    label: Option<String>,
    bounds: NativeAutomationBounds,
    value: Option<String>,
    selected: bool,
    children: Vec<NativeAutomationNodeSnapshot>,
) -> NativeAutomationNodeSnapshot {
    NativeAutomationNodeSnapshot {
        id: NativeAutomationNodeId::new(id),
        role,
        label,
        bounds,
        value,
        enabled: true,
        selected,
        available_actions: Vec::new(),
        metadata: BTreeMap::new(),
        children,
    }
}

fn bounds(x: f32, y: f32, width: f32, height: f32) -> NativeAutomationBounds {
    NativeAutomationBounds {
        x,
        y,
        width: width.max(0.0),
        height: height.max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_timing_artifact_uses_generic_radiant_runtime_export() {
        let module = include_str!("mod.rs");
        let legacy_compat_export = concat!(
            "pub use radiant::compat::",
            "legacy_shell::NativeStartupTimingArtifact;"
        );

        assert!(module.contains("pub use radiant::gui_runtime::NativeStartupTimingArtifact;"));
        assert!(
            !module.contains(legacy_compat_export),
            "startup timing is a generic Radiant runtime artifact, not a Wavecrate compat DTO"
        );
    }

    #[test]
    fn automation_snapshot_adapter_exposes_shell_root_from_wavecrate_model() {
        let model = NativeAppModel::default();
        let snapshot = capture_gui_automation_snapshot([1440.0, 810.0], &model);

        assert_eq!(snapshot.root.id.0, "shell.root");
        assert_eq!(snapshot.viewport_width, 1440);
        assert_eq!(snapshot.viewport_height, 810);
    }
}
