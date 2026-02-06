//! Backend-neutral application-core helpers shared by GUI runtimes.
//!
//! This module centralizes runtime-facing projection logic so renderer backends
//! do not depend directly on framework-specific UI modules.

pub(crate) mod native_shell;
