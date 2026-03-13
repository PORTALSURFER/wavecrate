//! Shared automation-snapshot lookup helpers for scenario assertions and AIV wrappers.

use crate::app_core::actions::{NativeAutomationNodeSnapshot, NativeGuiAutomationSnapshot};
use serde::Serialize;
use std::{fs, path::Path};

/// Window-relative semantic target resolved from one automation snapshot node.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GuiAutomationTarget {
    /// Stable semantic node identifier.
    pub node_id: String,
    /// Resolved node role string for machine-readable consumers.
    pub role: String,
    /// Center X coordinate in logical window space.
    pub center_x: f32,
    /// Center Y coordinate in logical window space.
    pub center_y: f32,
    /// Width in logical window space.
    pub width: f32,
    /// Height in logical window space.
    pub height: f32,
    /// Horizontal percentage within the captured viewport.
    pub x_percent: f32,
    /// Vertical percentage within the captured viewport.
    pub y_percent: f32,
    /// Whether the node is currently enabled.
    pub enabled: bool,
    /// Whether the node is currently selected.
    pub selected: bool,
    /// Stable action ids advertised by the node.
    pub available_actions: Vec<String>,
}

/// Find one semantic automation node by stable id.
pub(crate) fn find_automation_node<'a>(
    snapshot: &'a NativeGuiAutomationSnapshot,
    node_id: &str,
) -> Option<&'a NativeAutomationNodeSnapshot> {
    find_node_recursive(&snapshot.root, node_id)
}

/// Resolve one semantic automation target from a full snapshot.
pub fn resolve_automation_target(
    snapshot: &NativeGuiAutomationSnapshot,
    node_id: &str,
) -> Result<GuiAutomationTarget, String> {
    let node = find_automation_node(snapshot, node_id)
        .ok_or_else(|| format!("missing automation node {node_id}"))?;
    let center_x = node.bounds.x + node.bounds.width / 2.0;
    let center_y = node.bounds.y + node.bounds.height / 2.0;
    let viewport_width = snapshot.viewport_width.max(1) as f32;
    let viewport_height = snapshot.viewport_height.max(1) as f32;
    Ok(GuiAutomationTarget {
        node_id: node.id.0.clone(),
        role: format!("{:?}", node.role).to_ascii_lowercase(),
        center_x,
        center_y,
        width: node.bounds.width,
        height: node.bounds.height,
        x_percent: ((center_x / viewport_width) * 100.0).clamp(0.0, 100.0),
        y_percent: ((center_y / viewport_height) * 100.0).clamp(0.0, 100.0),
        enabled: node.enabled,
        selected: node.selected,
        available_actions: node.available_actions.clone(),
    })
}

/// Read a full automation snapshot from one GUI artifact bundle file.
pub fn read_automation_snapshot_from_artifact(
    artifact_path: &Path,
) -> Result<NativeGuiAutomationSnapshot, String> {
    let json = fs::read_to_string(artifact_path)
        .map_err(|err| format!("read GUI artifact {}: {err}", artifact_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|err| format!("parse GUI artifact {}: {err}", artifact_path.display()))?;
    let snapshot = value.get("automation_snapshot").cloned().ok_or_else(|| {
        format!(
            "GUI artifact {} is missing automation_snapshot",
            artifact_path.display()
        )
    })?;
    serde_json::from_value(snapshot).map_err(|err| {
        format!(
            "parse automation snapshot from GUI artifact {}: {err}",
            artifact_path.display()
        )
    })
}

fn find_node_recursive<'a>(
    node: &'a NativeAutomationNodeSnapshot,
    node_id: &str,
) -> Option<&'a NativeAutomationNodeSnapshot> {
    if node.id.0 == node_id {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_node_recursive(child, node_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_test::capture_default_bundle;

    #[test]
    fn resolve_target_uses_window_relative_center() {
        let bundle =
            capture_default_bundle(&crate::gui_test::GuiTestModeConfig::default()).expect("bundle");
        let target = resolve_automation_target(&bundle.automation_snapshot, "shell.root")
            .expect("root target");
        assert!(target.center_x > 0.0);
        assert!(target.center_y > 0.0);
        assert_eq!(target.node_id, "shell.root");
    }
}
