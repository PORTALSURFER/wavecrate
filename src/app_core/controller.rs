//! Backend-neutral controller aliases for migration consumers.
//!
//! The GUI migration still uses the legacy controller implementation internally,
//! but exposing it through `app_core` gives runtimes and tooling a stable path
//! that remains valid while `app` internals are retired.

/// Transitional controller type used by native runtime bridges and migration CLIs.
pub type AppController = crate::app::controller::EguiController;

/// Backend-neutral status helpers for migration-facing runtime code.
pub trait AppControllerStatusExt {
    /// Set an error status message on the controller.
    ///
    /// This keeps native-bridge code independent from legacy UI style enums
    /// while migration is in progress.
    fn set_error_status(&mut self, message: impl Into<String>);
}

impl AppControllerStatusExt for AppController {
    fn set_error_status(&mut self, message: impl Into<String>) {
        AppController::set_error_status(self, message);
    }
}
