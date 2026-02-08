//! Native runtime controller fallbacks when legacy runtime is disabled.
//!
//! This module keeps migration-facing APIs buildable under
//! `--no-default-features` while legacy controller internals are unavailable.

use std::{cell::RefCell, rc::Rc};

use crate::{
    app_core::actions::{NativeAppModel, NativeUiAction},
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};

/// Placeholder controller type for non-legacy builds.
#[derive(Default)]
pub struct AppController;

/// Build a migration-facing controller.
///
/// Returns an error when the legacy runtime feature is disabled.
pub fn build_native_app_controller(
    _renderer: WaveformRenderer,
    _player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<AppController, String> {
    Err(String::from(
        "legacy runtime is disabled (enable `legacy-egui-runtime`)",
    ))
}

/// Backend-neutral native-runtime orchestration helpers.
pub trait AppControllerNativeRuntimeExt {
    /// Apply per-frame controller maintenance before projecting the UI model.
    fn prepare_native_frame(&mut self);

    /// Project the current controller state into a native runtime app model.
    fn project_native_app_model(&mut self) -> NativeAppModel;

    /// Persist full configuration during native runtime shutdown.
    fn persist_native_exit_config(&self) -> Result<(), String>;

    /// Apply a native runtime UI action to the controller.
    fn apply_native_ui_action(&mut self, action: NativeUiAction);
}

impl AppControllerNativeRuntimeExt for AppController {
    fn prepare_native_frame(&mut self) {}

    fn project_native_app_model(&mut self) -> NativeAppModel {
        NativeAppModel::default()
    }

    fn persist_native_exit_config(&self) -> Result<(), String> {
        Ok(())
    }

    fn apply_native_ui_action(&mut self, _action: NativeUiAction) {}
}
