//! Legacy `app` module adapters for migration-facing `app_core` code.
//!
//! This module defines the explicit compatibility boundary: migration-facing
//! runtime code should depend on `app_core`, and only these adapters may touch
//! legacy runtime modules while extraction is in progress.

/// Legacy controller adapter surface.
pub(crate) mod controller;
/// Legacy state adapter surface.
pub(crate) mod state;
/// Legacy view-model adapter surface.
pub(crate) mod view_model;
