//! Backend-neutral controller aliases for migration consumers.
//!
//! The GUI migration still uses the legacy controller implementation internally,
//! but exposing it through `app_core` gives runtimes and tooling a stable path
//! that remains valid while `app` internals are retired.

/// Transitional controller type used by native runtime bridges and migration CLIs.
pub type AppController = crate::app::controller::LegacyAppController;

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

/// Backend-neutral native-runtime orchestration helpers.
pub trait AppControllerNativeRuntimeExt {
    /// Apply per-frame controller maintenance before projecting the UI model.
    fn prepare_native_frame(&mut self);

    /// Project the current controller state into a native runtime app model.
    fn project_native_app_model(&mut self) -> radiant::app::AppModel;

    /// Persist full configuration during native runtime shutdown.
    fn persist_native_exit_config(&self) -> Result<(), String>;
}

impl AppControllerNativeRuntimeExt for AppController {
    fn prepare_native_frame(&mut self) {
        self.tick_playhead();
        self.poll_background_jobs();
        self.update_performance_governor(false);
    }

    fn project_native_app_model(&mut self) -> radiant::app::AppModel {
        crate::app_core::native_shell::project_app_model(self)
    }

    fn persist_native_exit_config(&self) -> Result<(), String> {
        self.save_full_config()
            .map_err(|err| format!("Failed to persist config on native runtime exit: {err}"))
    }
}
