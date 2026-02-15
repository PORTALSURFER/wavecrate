//! Transitional aliases for legacy `crate::app` modules.
//!
//! This module centralizes temporary migration boundaries so host-facing
//! `app_core` code depends on one explicit alias layer instead of reaching into
//! legacy modules directly.
//!
//! The shim is temporary and should be removed once migration-facing host code
//! no longer requires direct access to `crate::app` state, view-model, or
//! controller internals.

/// Boundary generation for migration tracking; increase this when the shim
/// expands or its removal criteria changes.
pub(crate) const MIGRATION_BOUNDARY_GENERATION: u32 = 1;

/// Reasonable removal target for this shim once migration parity is complete.
pub(crate) const MIGRATION_BOUNDARY_REMOVAL_PLAN: &str =
    "Remove after no migration-facing module imports `crate::app` directly.";

/// Legacy controller module alias used by migration bridge shims.
pub(crate) mod controller {
    pub(crate) use crate::app::controller::AppController;
}

/// Legacy state module alias used by migration-facing state converters.
pub(crate) mod state {
    pub(crate) use crate::app::state;
}

/// Legacy view-model module alias used by migration-facing formatting helpers.
pub(crate) mod view_model {
    pub(crate) use crate::app::view_model;
}
