//! Semantic assertion helpers for the in-process GUI scenario runner.

use crate::{
    app_core::actions::{NativeGuiAutomationSnapshot, action_catalog_entry_by_id},
    gui_test::{GuiActionTraceEvent, GuiAssertion, find_automation_node},
};

/// Evaluate one semantic GUI assertion against the latest snapshot and action trace.
pub(super) fn assert_scenario_state(
    snapshot: &NativeGuiAutomationSnapshot,
    trace: &[GuiActionTraceEvent],
    assertion: &GuiAssertion,
) -> Result<(), String> {
    match assertion {
        GuiAssertion::NodePresent { node_id } => find_automation_node(snapshot, node_id)
            .map(|_| ())
            .ok_or_else(|| format!("missing automation node {node_id}")),
        GuiAssertion::NodeAbsent { node_id } => find_automation_node(snapshot, node_id)
            .is_none()
            .then_some(())
            .ok_or_else(|| format!("unexpected automation node {node_id}")),
        GuiAssertion::NodeSelected { node_id, selected } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            if node.selected == *selected {
                Ok(())
            } else {
                Err(format!(
                    "automation node {node_id} selected={} expected={selected}",
                    node.selected
                ))
            }
        }
        GuiAssertion::NodeEnabled { node_id, enabled } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            if node.enabled == *enabled {
                Ok(())
            } else {
                Err(format!(
                    "automation node {node_id} enabled={} expected={enabled}",
                    node.enabled
                ))
            }
        }
        GuiAssertion::NodeValueContains { node_id, needle } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            let value = node.value.as_deref().unwrap_or_default();
            value.contains(needle).then_some(()).ok_or_else(|| {
                format!("automation node {node_id} value '{value}' does not contain '{needle}'")
            })
        }
        GuiAssertion::NodeActionAvailable { node_id, action_id } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            node.available_actions
                .iter()
                .any(|available| available == action_id)
                .then_some(())
                .ok_or_else(|| {
                    format!("automation node {node_id} does not advertise action {action_id}")
                })
        }
        GuiAssertion::NodeMetadataContains {
            node_id,
            key,
            needle,
        } => {
            let node = find_automation_node(snapshot, node_id)
                .ok_or_else(|| format!("missing automation node {node_id}"))?;
            let actual = node
                .metadata
                .get(key)
                .map(String::as_str)
                .unwrap_or_default();
            actual
                .contains(needle)
                .then_some(())
                .ok_or_else(|| {
                    format!(
                        "automation node {node_id} metadata {key}='{actual}' does not contain '{needle}'"
                    )
                })
        }
        GuiAssertion::ActionCataloged { action_id } => action_catalog_entry_by_id(action_id)
            .map(|_| ())
            .ok_or_else(|| format!("missing catalog action {action_id}")),
        GuiAssertion::ActionRecorded { action_id } => trace
            .iter()
            .any(|event| event.action_id == *action_id)
            .then_some(())
            .ok_or_else(|| format!("action trace does not contain {action_id}")),
    }
}
