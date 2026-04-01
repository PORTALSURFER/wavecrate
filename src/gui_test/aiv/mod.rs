//! Typed desktop AIV suite manifests exported for the PowerShell desktop runner.

mod packs;

use super::GuiAutomationTarget;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub use self::packs::gui_aiv_suite_manifest;

/// Backward-compatible default desktop AIV pack name.
pub const DEFAULT_GUI_AIV_PACK: &str = "desktop-smoke";
/// Full desktop regression pack name used by the PowerShell suite runner.
pub const REGRESSION_GUI_AIV_PACK: &str = "desktop-regression";
pub(super) const GUI_AIV_SCHEMA_VERSION: u32 = 1;
pub(super) const GUI_TEST_WINDOW_TITLE: &str = "Sempal GUI Test";
pub(super) const DEFAULT_VIEWPORT: [u32; 2] = [1440, 810];

/// Named desktop AIV manifest exported for the Windows PowerShell runner.
///
/// The manifest stays semantic-first: every case declares stable automation node
/// ids and expected semantic assertions, while screenshots remain diagnostic
/// artifacts owned by the desktop wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiAivSuiteManifest {
    /// Schema version for forward-compatible manifest readers.
    pub schema_version: u32,
    /// Stable desktop suite pack name.
    pub pack_name: String,
    /// Default desktop window title the runner should focus.
    pub window_title: String,
    /// Ordered desktop cases executed by the runner.
    pub cases: Vec<GuiAivCase>,
}

/// One isolated desktop GUI regression case executed through AIV.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiAivCase {
    /// Stable case identifier used for artifacts and reports.
    pub name: String,
    /// Deterministic fixture tag passed to GUI test mode for this case.
    pub fixture_tag: String,
    /// Fixed logical viewport requested for the launched window.
    pub viewport: [u32; 2],
    /// Expected native window title for focus and click targeting.
    pub window_title: String,
    /// Ordered desktop steps executed by the PowerShell runner.
    pub steps: Vec<GuiAivStep>,
    /// Final semantic assertions evaluated after the step sequence completes.
    pub expected_assertions: Vec<GuiAivAssertion>,
}

/// One deterministic desktop step supported by the AIV wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuiAivStep {
    /// Wait until the requested semantic node becomes visible in the latest GUI artifact.
    WaitForNode {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Maximum wait duration before the runner fails the case.
        timeout_ms: u64,
    },
    /// Click one semantic node, optionally at a node-relative percentage offset.
    ClickNode {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Optional horizontal click point inside the node bounds (`0..=100`).
        x_percent: Option<u8>,
        /// Optional vertical click point inside the node bounds (`0..=100`).
        y_percent: Option<u8>,
    },
    /// Focus one semantic input field and type replacement text into it.
    TypeIntoNode {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Full text payload to type.
        text: String,
        /// Whether the runner should clear the field before typing.
        clear_existing: bool,
    },
    /// Press one keyboard key with optional modifiers.
    PressKey {
        /// Key name forwarded to `aiv keyboard key`.
        key: String,
        /// Whether Ctrl should be held during the key press.
        ctrl: bool,
        /// Whether Alt should be held during the key press.
        alt: bool,
        /// Whether Shift should be held during the key press.
        shift: bool,
    },
    /// Drag inside one semantic node between two node-relative points.
    DragInNode {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Horizontal start point inside the node bounds (`0..=100`).
        start_x_percent: u8,
        /// Vertical start point inside the node bounds (`0..=100`).
        start_y_percent: u8,
        /// Horizontal end point inside the node bounds (`0..=100`).
        end_x_percent: u8,
        /// Vertical end point inside the node bounds (`0..=100`).
        end_y_percent: u8,
    },
    /// Scroll inside one semantic node from an optional node-relative anchor point.
    ScrollInNode {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Vertical scroll delta forwarded to AIV.
        delta: i32,
        /// Optional horizontal anchor inside the node bounds (`0..=100`).
        x_percent: Option<u8>,
        /// Optional vertical anchor inside the node bounds (`0..=100`).
        y_percent: Option<u8>,
    },
    /// Capture a diagnostic screenshot without using it as the test oracle.
    CaptureScreenshot {
        /// Stable screenshot label used in case artifacts.
        label: String,
    },
    /// Evaluate one semantic assertion immediately after the step.
    Assert {
        /// Semantic assertion evaluated against the latest GUI artifact.
        assertion: GuiAivAssertion,
    },
}

/// Semantic assertion evaluated from `gui_test_latest.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuiAivAssertion {
    /// Assert that one semantic automation node exists.
    AssertNodePresent {
        /// Stable semantic automation node identifier.
        node_id: String,
    },
    /// Assert that one semantic automation node does not exist.
    AssertNodeAbsent {
        /// Stable semantic automation node identifier.
        node_id: String,
    },
    /// Assert that one semantic automation node has the requested selected state.
    AssertNodeSelected {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Expected selected flag.
        selected: bool,
    },
    /// Assert that one semantic automation node value contains the requested text.
    AssertNodeValueContains {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Expected text fragment.
        needle: String,
    },
    /// Assert that one semantic automation node metadata value contains the requested text.
    AssertNodeMetadataContains {
        /// Stable semantic automation node identifier.
        node_id: String,
        /// Stable metadata key exposed by the automation snapshot.
        key: String,
        /// Expected text fragment inside the metadata value.
        needle: String,
    },
    /// Assert that the live action trace contains one handled stable action id.
    AssertActionRecorded {
        /// Stable action identifier from the GUI action catalog.
        action_id: String,
    },
}

impl GuiAutomationTarget {
    /// Resolve one node-relative logical point inside the target bounds.
    ///
    /// Percentages are clamped into `0..=100` so desktop wrappers can recover
    /// safely from manifest mistakes without clicking outside the node.
    pub fn point_at_percent(&self, x_percent: u8, y_percent: u8) -> [f32; 2] {
        let left = self.center_x - (self.width * 0.5);
        let top = self.center_y - (self.height * 0.5);
        let x_ratio = f32::from(x_percent.min(100)) / 100.0;
        let y_ratio = f32::from(y_percent.min(100)) / 100.0;
        [left + (self.width * x_ratio), top + (self.height * y_ratio)]
    }
}

/// Export the backward-compatible default desktop AIV suite (`desktop-smoke`).
pub fn export_aiv_suite(output_path: &Path) -> Result<(), String> {
    export_aiv_suite_pack(DEFAULT_GUI_AIV_PACK, output_path)
}

/// Export one named desktop AIV suite manifest for the PowerShell runner.
pub fn export_aiv_suite_pack(pack_name: &str, output_path: &Path) -> Result<(), String> {
    let manifest = gui_aiv_suite_manifest(pack_name)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create desktop AIV suite directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|err| format!("failed to serialize desktop AIV suite: {err}"))?;
    fs::write(output_path, json).map_err(|err| {
        format!(
            "failed to write desktop AIV suite {}: {err}",
            output_path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_at_percent_clamps_to_node_bounds() {
        let target = GuiAutomationTarget {
            node_id: String::from("waveform.region"),
            role: String::from("waveform_region"),
            center_x: 110.0,
            center_y: 70.0,
            width: 80.0,
            height: 40.0,
            x_percent: 0.0,
            y_percent: 0.0,
            enabled: true,
            selected: false,
            available_actions: Vec::new(),
        };
        assert_eq!(target.point_at_percent(0, 0), [70.0, 50.0]);
        assert_eq!(target.point_at_percent(50, 50), [110.0, 70.0]);
        assert_eq!(target.point_at_percent(100, 100), [150.0, 90.0]);
    }
}
