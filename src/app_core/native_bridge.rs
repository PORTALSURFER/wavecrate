//! Native runtime bridge implementations for migration-facing runtimes.
//!
//! This module hosts the `radiant` bridge surface so runtime entrypoints can
//! depend on `app_core` instead of legacy runtime module paths.

use crate::{
    app_core::actions::{NativeAppModel, NativeFrameBuildResult, NativeUiAction},
    app_core::actions::NativeAppBridge,
    app_core::controller::{
        AppController, AppControllerNativeRuntimeExt, build_native_app_controller,
    },
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
use std::{cell::RefCell, rc::Rc};

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: AppController,
}

impl SempalNativeBridge {
    /// Build a new native bridge initialized with persisted sempal configuration.
    pub fn new(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
    ) -> Result<Self, String> {
        let controller = build_native_app_controller(renderer, player)?;
        Ok(Self { controller })
    }
}

impl NativeAppBridge for SempalNativeBridge {
    fn pull_model(&mut self) -> NativeAppModel {
        self.controller.prepare_native_frame();
        self.controller.project_native_app_model()
    }

    fn on_action(&mut self, action: NativeUiAction) {
        self.controller.apply_native_ui_action(action);
    }

    fn on_frame_result(&mut self, _result: NativeFrameBuildResult) {}

    fn on_exit(&mut self) {
        if let Err(err) = self.controller.persist_native_exit_config() {
            eprintln!("{err}");
        }
    }
}

/// Construct a native runtime bridge for the current sempal controller stack.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<SempalNativeBridge, String> {
    SempalNativeBridge::new(renderer, player)
}
