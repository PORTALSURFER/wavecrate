//! Declarative GUI scenario and assertion contracts used by the CLI runner.

use crate::app_core::actions::NativeUiAction;
use serde::{Deserialize, Serialize};

/// Declarative GUI scenario executed by the in-process GUI test runner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuiScenario {
    /// Human-readable scenario name.
    pub name: String,
    /// Fixture tag associated with the scenario.
    pub fixture_tag: String,
    /// Ordered scenario steps.
    pub steps: Vec<GuiScenarioStep>,
}

/// One executable step in a declarative GUI scenario.
///
/// Scenario runs intentionally support only deterministic action dispatch and
/// semantic assertions. Snapshot capture lives on the dedicated `snapshot`
/// command instead of as an in-band scenario step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuiScenarioStep {
    /// Dispatch a concrete native UI action.
    DispatchAction {
        /// Concrete native UI action to apply to the in-process GUI harness.
        action: NativeUiAction,
    },
    /// Evaluate a deterministic semantic assertion.
    Assert {
        /// Semantic assertion to evaluate against the latest automation snapshot.
        assertion: GuiAssertion,
    },
}

/// Deterministic assertion supported by the in-process GUI scenario runner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuiAssertion {
    /// Assert that a semantic automation node exists.
    NodePresent {
        /// Stable automation node identifier expected to exist.
        node_id: String,
    },
    /// Assert that a semantic automation node does not exist.
    NodeAbsent {
        /// Stable automation node identifier expected to be absent.
        node_id: String,
    },
    /// Assert that a semantic automation node has the requested selected state.
    NodeSelected {
        /// Stable automation node identifier to inspect.
        node_id: String,
        /// Expected selected state for the targeted node.
        selected: bool,
    },
    /// Assert that a semantic automation node has the requested enabled state.
    NodeEnabled {
        /// Stable automation node identifier to inspect.
        node_id: String,
        /// Expected enabled state for the targeted node.
        enabled: bool,
    },
    /// Assert that a semantic automation node value contains the requested text.
    NodeValueContains {
        /// Stable automation node identifier to inspect.
        node_id: String,
        /// Text fragment expected within the node value.
        needle: String,
    },
    /// Assert that a semantic automation node advertises one stable action id.
    NodeActionAvailable {
        /// Stable automation node identifier to inspect.
        node_id: String,
        /// Stable action identifier expected on the node.
        action_id: String,
    },
    /// Assert that a semantic automation node metadata value contains the requested text.
    NodeMetadataContains {
        /// Stable automation node identifier to inspect.
        node_id: String,
        /// Metadata key expected on the node.
        key: String,
        /// Text fragment expected within the metadata value.
        needle: String,
    },
    /// Assert that the stable action id is present in the host action catalog.
    ActionCataloged {
        /// Stable action identifier expected in the GUI action catalog.
        action_id: String,
    },
    /// Assert that the scenario action trace already contains one handled stable action id.
    ActionRecorded {
        /// Stable action identifier expected in the in-process scenario trace.
        action_id: String,
    },
}
