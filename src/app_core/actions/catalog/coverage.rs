//! Coverage-oriented metadata for the host-owned GUI action catalog.

use super::GuiActionKind;
use serde::Serialize;

/// GUI ownership surface used for coverage and automation planning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiSurface {
    /// Browser list, tabs, filters, and related sample browsing controls.
    Browser,
    /// Source and folder management controls.
    Sources,
    /// Waveform view, transport-adjacent edits, and zoom/selection controls.
    Waveform,
    /// Global playback transport and volume controls.
    Transport,
    /// Two-dimensional sample map surface.
    Map,
    /// Options and settings panel surface.
    Options,
    /// Prompt or confirmation dialog surface.
    Prompt,
    /// Update notification and installer surface.
    Update,
}

/// Expected effect class used to group contract expectations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiEffectClass {
    /// Action mutates state without expected background IO or visible motion.
    StateOnly,
    /// Action rebuilds or materially changes the projected UI model.
    Projection,
    /// Action is primarily a high-frequency runtime-motion interaction.
    RuntimeMotion,
    /// Action starts or interacts with an IO-backed job or background task.
    IoJob,
    /// Action can delete or irreversibly modify user data.
    Destructive,
}

/// Required coverage layer for one GUI action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiCoverageLayer {
    /// Semantic automation snapshot coverage for stable node/action contracts.
    SemanticContract,
    /// Native runtime input-routing coverage.
    RuntimeInput,
    /// App-core or bridge projection snapshot coverage.
    ProjectionSnapshot,
    /// Desktop AIV coverage against the live Windows application.
    DesktopAiv,
}

/// Undo/redo transaction policy for one GUI action in the v1 history model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiHistoryPolicy {
    /// Action is intentionally excluded from transactional history.
    None,
    /// Action should commit one synchronous undoable transaction immediately.
    Immediate,
    /// Action starts async or IO-backed work and records history on success.
    Deferred,
}

/// Host-owned coverage metadata for one `UiAction` variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct GuiActionCatalogEntry {
    /// Stable payload-free action identity.
    pub kind: GuiActionKind,
    /// Stable action identifier used in reports and automation metadata.
    pub action_id: &'static str,
    /// Top-level GUI ownership surface for the action.
    pub surface: GuiSurface,
    /// Expected effect class for the action.
    pub effect_class: GuiEffectClass,
    /// Undo/redo transaction policy for the action.
    pub history_policy: GuiHistoryPolicy,
    /// Coverage layers that must exist for the action.
    pub coverage_layers: &'static [GuiCoverageLayer],
    /// Default fixture or scenario tags to seed targeted suites.
    pub default_fixture_tags: &'static [&'static str],
}
