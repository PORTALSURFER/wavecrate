//! Shared automation-snapshot lookup helpers for scenario assertions.

use radiant::gui::automation::{AutomationNodeSnapshot, GuiAutomationSnapshot};
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
    snapshot: &'a GuiAutomationSnapshot,
    node_id: &str,
) -> Option<&'a AutomationNodeSnapshot> {
    find_node_recursive(&snapshot.root, node_id)
}

/// Resolve one semantic automation target from a full snapshot.
pub fn resolve_automation_target(
    snapshot: &GuiAutomationSnapshot,
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
) -> Result<GuiAutomationSnapshot, String> {
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
    node: &'a AutomationNodeSnapshot,
    node_id: &str,
) -> Option<&'a AutomationNodeSnapshot> {
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
    use crate::{
        app_dirs::{APP_DIR_NAME, AppRootGuard, PersistenceProfileGuard},
        gui_test::{GuiFixtureRuntime, GuiTestModeConfig, capture_default_bundle},
        sample_sources::config::{AppConfig, AppSettingsCore, config_path, save},
    };
    use radiant::gui::automation::{
        AutomationBounds, AutomationNodeId, AutomationNodeSemantics, AutomationNodeSnapshot,
        AutomationRole, GuiAutomationSnapshot,
    };
    use tempfile::tempdir;

    fn sample_snapshot() -> GuiAutomationSnapshot {
        GuiAutomationSnapshot {
            schema_version: 1,
            viewport_width: 400,
            viewport_height: 200,
            root: AutomationNodeSnapshot::from_semantics(
                AutomationNodeId::new("shell.root"),
                AutomationBounds {
                    x: 0.0,
                    y: 0.0,
                    width: 400.0,
                    height: 200.0,
                },
                AutomationNodeSemantics::new(AutomationRole::Root).with_label("Root"),
            ),
        }
    }

    #[test]
    fn resolve_target_uses_window_relative_center() {
        let live_base = tempdir().expect("live config base");
        let _live_guard = PersistenceProfileGuard::live();
        let _root_guard = AppRootGuard::set(live_base.path().join(APP_DIR_NAME))
            .expect("select isolated live sentinel root");
        save(&AppConfig {
            sources: Vec::new(),
            core: AppSettingsCore {
                volume: 0.314,
                ..AppSettingsCore::default()
            },
        })
        .expect("seed live sentinel config");
        let live_config_path = config_path().expect("live config path");
        let live_before = std::fs::read(&live_config_path).expect("read live config");

        let bundle = capture_default_bundle(&GuiTestModeConfig::default()).expect("bundle");

        assert_eq!(bundle.fixture_runtime, GuiFixtureRuntime::NativeApp);
        assert_eq!(
            bundle
                .runtime_composition
                .expect("native runtime composition"),
            crate::gui_test::GuiRuntimeComposition {
                native_source_watchers: 1,
                native_readiness_supervisors: 1,
                legacy_analysis_pools: 0,
            }
        );
        assert_eq!(
            bundle
                .shutdown_artifact
                .as_ref()
                .and_then(|artifact| artifact["source_processing"]["joined"].as_bool()),
            Some(true)
        );
        assert_eq!(
            std::fs::read(live_config_path).expect("re-read live config"),
            live_before,
            "isolated native automation must not mutate the live profile"
        );
        let target_id = bundle
            .automation_snapshot
            .root
            .automation_targets()
            .into_iter()
            .find(|target| !target.bounds.is_empty())
            .map(|target| target.id.0)
            .expect("visible native automation target");
        let target = resolve_automation_target(&bundle.automation_snapshot, &target_id)
            .expect("visible target");
        assert!(target.center_x > 0.0);
        assert!(target.center_y > 0.0);
        assert_eq!(target.node_id, target_id);
    }

    #[test]
    fn small_multi_source_fixture_uses_native_workers() {
        let bundle = capture_default_bundle(&GuiTestModeConfig {
            fixture_tag: String::from(crate::gui_test::GUI_TEST_SMALL_MULTI_SOURCE_FIXTURE_TAG),
            ..GuiTestModeConfig::default()
        })
        .expect("small native source fixture bundle");

        assert_eq!(bundle.fixture_runtime, GuiFixtureRuntime::NativeApp);
        assert_eq!(
            bundle.runtime_composition,
            Some(crate::gui_test::GuiRuntimeComposition {
                native_source_watchers: 1,
                native_readiness_supervisors: 1,
                legacy_analysis_pools: 0,
            })
        );
        assert_eq!(bundle.fixture_tag, "small-multi-source");
        assert_eq!(
            bundle
                .shutdown_artifact
                .as_ref()
                .and_then(|artifact| artifact["source_processing"]["joined"].as_bool()),
            Some(true)
        );
    }

    #[test]
    fn resolve_target_reports_missing_node() {
        let err = resolve_automation_target(&sample_snapshot(), "missing.node").unwrap_err();
        assert!(err.contains("missing automation node missing.node"));
    }

    #[test]
    fn read_snapshot_from_artifact_rejects_missing_snapshot_field() {
        let temp = tempdir().expect("tempdir");
        let artifact = temp.path().join("artifact.json");
        std::fs::write(&artifact, r#"{"schema_version":1}"#).expect("write artifact");

        let err = read_automation_snapshot_from_artifact(&artifact).unwrap_err();
        assert!(err.contains("missing automation_snapshot"));
    }

    #[test]
    fn read_snapshot_from_artifact_rejects_malformed_artifact_json() {
        let temp = tempdir().expect("tempdir");
        let artifact = temp.path().join("artifact.json");
        std::fs::write(&artifact, "{not-json").expect("write artifact");

        let err = read_automation_snapshot_from_artifact(&artifact).unwrap_err();
        assert!(err.contains("parse GUI artifact"));
    }

    #[test]
    fn read_snapshot_from_artifact_rejects_malformed_snapshot_payload() {
        let temp = tempdir().expect("tempdir");
        let artifact = temp.path().join("artifact.json");
        std::fs::write(&artifact, r#"{"automation_snapshot":3}"#).expect("write artifact");

        let err = read_automation_snapshot_from_artifact(&artifact).unwrap_err();
        assert!(err.contains("parse automation snapshot"));
    }
}
