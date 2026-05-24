//! Shared helper utilities for semantic automation snapshot builders.

use super::*;
use crate::app_core::native_shell::runtime_contract::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    NormalizedRangeModel, UpdateStatusModel,
};
use std::collections::BTreeMap;

/// Build one leaf automation node with empty metadata and no children.
pub(super) fn simple_node(
    id: impl Into<String>,
    role: AutomationRole,
    label: Option<String>,
    rect: Rect,
    value: Option<String>,
    enabled: bool,
    selected: bool,
    available_actions: Vec<String>,
) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot {
        id: node_id(id),
        role,
        label,
        bounds: bounds(rect),
        value,
        enabled,
        selected,
        available_actions,
        metadata: BTreeMap::new(),
        children: Vec::new(),
    }
}

/// Build one stable automation node id.
pub(super) fn node_id(id: impl Into<String>) -> AutomationNodeId {
    AutomationNodeId::new(id)
}

/// Convert one UI rectangle into deterministic automation bounds.
pub(super) fn bounds(rect: Rect) -> AutomationBounds {
    AutomationBounds {
        x: quantize(rect.min.x),
        y: quantize(rect.min.y),
        width: quantize(rect.width()),
        height: quantize(rect.height()),
    }
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

/// Build metadata while omitting empty values.
pub(super) fn metadata(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| (String::from(*key), String::from(*value)))
        .collect()
}

/// Return a stable textual boolean for semantic metadata.
pub(super) fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

/// Return a stable update-status label for semantic metadata.
pub(super) fn update_status_text(status: UpdateStatusModel) -> &'static str {
    match status {
        UpdateStatusModel::Idle => "idle",
        UpdateStatusModel::Checking => "checking",
        UpdateStatusModel::Available => "available",
        UpdateStatusModel::Error => "error",
    }
}

/// Return a stable selection range string in microseconds.
pub(super) fn selection_micros_text(range: NormalizedRangeModel) -> String {
    format!("{}-{}", range.start_micros, range.end_micros)
}

/// Convert a UI label into a stable automation slug.
pub(super) fn slug(label: &str) -> String {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Return a square rectangle centered on the supplied point.
pub(super) fn circle_rect(center: Point, diameter: f32) -> Rect {
    let radius = diameter * 0.5;
    Rect::from_min_max(
        Point::new(center.x - radius, center.y - radius),
        Point::new(center.x + radius, center.y + radius),
    )
}
