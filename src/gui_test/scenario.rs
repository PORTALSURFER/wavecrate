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
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuiScenarioStep {
    /// Capture the latest automation snapshot without mutating state.
    CaptureSnapshot { label: String },
    /// Dispatch a concrete native UI action.
    DispatchAction { action: NativeUiAction },
    /// Evaluate a deterministic semantic assertion.
    Assert { assertion: GuiAssertion },
}

/// Deterministic assertion supported by the in-process GUI scenario runner.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuiAssertion {
    /// Assert that a semantic automation node exists.
    NodePresent { node_id: String },
    /// Assert that a semantic automation node does not exist.
    NodeAbsent { node_id: String },
    /// Assert that a semantic automation node has the requested selected state.
    NodeSelected { node_id: String, selected: bool },
    /// Assert that a semantic automation node has the requested enabled state.
    NodeEnabled { node_id: String, enabled: bool },
    /// Assert that a semantic automation node value contains the requested text.
    NodeValueContains { node_id: String, needle: String },
    /// Assert that a semantic automation node advertises one stable action id.
    NodeActionAvailable { node_id: String, action_id: String },
    /// Assert that a semantic automation node metadata value contains the requested text.
    NodeMetadataContains {
        node_id: String,
        key: String,
        needle: String,
    },
    /// Assert that the stable action id is present in the host action catalog.
    ActionCataloged { action_id: String },
}
