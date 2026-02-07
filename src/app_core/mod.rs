//! Backend-neutral application-core helpers shared by GUI runtimes.
//!
//! This module centralizes runtime-facing projection logic so renderer backends
//! do not depend directly on framework-specific UI modules.

/// Transitional controller aliases used by migration-facing runtimes and CLIs.
pub mod controller;
pub(crate) mod native_shell;
/// Transitional state aliases used by migration-facing runtimes and CLIs.
pub mod state;
