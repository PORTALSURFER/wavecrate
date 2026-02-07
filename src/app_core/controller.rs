//! Backend-neutral controller aliases for migration consumers.
//!
//! The GUI migration still uses the legacy controller implementation internally,
//! but exposing it through `app_core` gives runtimes and tooling a stable path
//! that remains valid while `egui_app` internals are retired.

/// Transitional controller type used by native runtime bridges and migration CLIs.
pub type AppController = crate::egui_app::controller::EguiController;
