//! Backend-neutral application-core helpers shared by GUI runtimes.
//!
//! This module centralizes runtime-facing projection logic so renderer backends
//! do not depend directly on framework-specific UI modules.

/// Transitional controller aliases used by migration-facing runtimes and CLIs.
#[cfg_attr(
    not(feature = "legacy-egui-runtime"),
    path = "controller_stub.rs"
)]
pub mod controller;
/// Native runtime action/model aliases for migration-facing glue code.
pub mod actions;
/// Native runtime bridge implementations for migration-facing runtimes.
pub mod native_bridge;
#[cfg(feature = "legacy-egui-runtime")]
pub(crate) mod native_shell;
/// Centralized aliases for remaining legacy `app` module dependencies.
#[cfg(feature = "legacy-egui-runtime")]
pub(crate) mod legacy;
/// Transitional state aliases used by migration-facing runtimes and CLIs.
#[cfg_attr(not(feature = "legacy-egui-runtime"), path = "state_stub.rs")]
pub mod state;
/// Transitional view-model helpers used by migration-facing projections.
#[cfg_attr(
    not(feature = "legacy-egui-runtime"),
    path = "view_model_stub.rs"
)]
pub mod view_model;
/// Transitional UI constants used by migration-facing runtimes.
pub mod ui;
