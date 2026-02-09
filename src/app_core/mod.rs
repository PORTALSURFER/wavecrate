//! Backend-neutral application-core helpers shared by GUI runtimes.
//!
//! This module centralizes runtime-facing projection logic so renderer backends
//! do not depend directly on framework-specific UI modules.

/// Transitional controller aliases used by migration-facing runtimes and CLIs.
pub mod controller;
/// Native runtime action/model aliases for migration-facing glue code.
pub mod actions;
/// Native runtime bridge implementations for migration-facing runtimes.
pub mod native_bridge;
pub(crate) mod native_shell;
/// Centralized aliases for remaining legacy `app` module dependencies.
pub(crate) mod legacy;
/// Transitional state aliases used by migration-facing runtimes and CLIs.
pub mod state;
/// Transitional view-model helpers used by migration-facing projections.
pub mod view_model;
/// Transitional UI constants used by migration-facing runtimes.
pub mod ui;
