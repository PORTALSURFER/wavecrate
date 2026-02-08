//! Minimal migration-facing state types when legacy runtime is disabled.
//!
//! These keep `app_core` APIs available for compile-time validation in
//! `--no-default-features` profiles.

use crate::app_core::actions::NativeBrowserTagTarget;

/// Placeholder UI state for non-legacy builds.
#[derive(Clone, Debug, Default)]
pub struct UiState;

/// Bounds used to query visible points in the map projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapQueryBounds {
    /// Minimum X coordinate for query.
    pub min_x: f32,
    /// Maximum X coordinate for query.
    pub max_x: f32,
    /// Minimum Y coordinate for query.
    pub min_y: f32,
    /// Maximum Y coordinate for query.
    pub max_y: f32,
}

/// Browser tab selection state used by migration-facing consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBrowserTab {
    /// List/table browser tab.
    List,
    /// Similarity map browser tab.
    Map,
}

/// Browser tag targets used by migration-facing action routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserTagTarget {
    /// Mark selection as trash.
    Trash,
    /// Mark selection as neutral.
    Neutral,
    /// Mark selection as keep.
    Keep,
}

impl From<NativeBrowserTagTarget> for BrowserTagTarget {
    fn from(value: NativeBrowserTagTarget) -> Self {
        match value {
            NativeBrowserTagTarget::Trash => Self::Trash,
            NativeBrowserTagTarget::Neutral => Self::Neutral,
            NativeBrowserTagTarget::Keep => Self::Keep,
        }
    }
}

/// Browser triage columns used in migration-facing drag/drop projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriageFlagColumn {
    /// Trash column.
    Trash,
    /// Neutral column.
    Neutral,
    /// Keep column.
    Keep,
}

/// Update status surfaced by migration-facing render projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    /// No update activity in progress.
    Idle,
    /// Update check in progress.
    Checking,
    /// A newer update is available.
    UpdateAvailable,
    /// Update check failed.
    Error,
}

/// Map render mode used by migration-facing render projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapRenderMode {
    /// Render a density heatmap.
    Heatmap,
    /// Render individual points.
    Points,
}
