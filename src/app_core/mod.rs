//! Backend-neutral application-core helpers shared by GUI runtimes.
//!
//! Migration-facing compatibility adapters are owned by focused app-core modules.

mod app_api;

/// Controller aliases and helpers used by migration-facing runtimes.
pub mod controller;

/// UI projection action/model aliases for migration-facing glue code.
pub mod actions;

/// Retained projection bridge implementations for migration-facing runtimes.
pub mod ui_bridge;
pub(crate) mod ui_projection;

/// Migration-facing state projections used by app bridge adapters.
pub mod state;

/// Migration-facing view-model helpers used by projections.
pub mod view_model;

/// Migration-facing UI constants used by runtime hosts.
pub mod ui;

#[cfg(test)]
pub(crate) mod test_fixtures;
