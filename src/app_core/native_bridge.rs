//! Native runtime bridge implementations for migration-facing runtimes.
//!
//! This module hosts the `radiant` bridge surface so runtime entrypoints can
//! depend on `app_core` instead of legacy runtime module paths.

use crate::{
    app_core::actions::NativeAppBridge,
    app_core::actions::NativeMotionModel,
    app_core::actions::{NativeAppModel, NativeFrameBuildResult, NativeUiAction},
    app_core::controller::{
        AppController, AppControllerNativeRuntimeExt, build_native_app_controller,
    },
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
use tracing::{error, info};

static PULL_MODEL_COUNT: AtomicU64 = AtomicU64::new(0);
static PULL_MOTION_COUNT: AtomicU64 = AtomicU64::new(0);
static ACTION_COUNT: AtomicU64 = AtomicU64::new(0);

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
        info!("Building native bridge controller");
        let controller = build_native_app_controller(renderer, player).map_err(|err| {
            error!(err = %err, "Failed to build native app controller");
            err
        })?;
        info!("Native bridge controller ready");
        Ok(Self { controller })
    }
}

impl NativeAppBridge for SempalNativeBridge {
    fn pull_model(&mut self) -> NativeAppModel {
        let call = PULL_MODEL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if call <= 24 {
            info!(call, "native bridge: pull_model start");
        }
        self.controller.prepare_native_frame();
        let model = self.controller.project_native_app_model();
        if call <= 24 {
            info!(
                call,
                transport_running = model.transport_running,
                browser_visible = model.browser.visible_count,
                status_len = model.status_text.len(),
                "native bridge: pull_model completed"
            );
        }
        model
    }

    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        let call = PULL_MOTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model start");
        }
        self.controller.prepare_native_frame();
        let model = Some(self.controller.project_native_motion_model());
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model completed");
        }
        model
    }

    fn on_action(&mut self, action: NativeUiAction) {
        let call = ACTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if call <= 64 {
            info!(call, action = ?action, "native bridge: on_action");
        }
        self.controller.apply_native_ui_action(action);
    }

    fn on_frame_result(&mut self, _result: NativeFrameBuildResult) {}

    fn on_exit(&mut self) {
        if let Err(err) = self.controller.persist_native_exit_config() {
            error!(err = %err, "Failed to persist config on native exit");
            eprintln!("{err}");
            return;
        }
        info!("Persisted config on native exit");
    }
}

/// Construct a native runtime bridge for the current sempal controller stack.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<SempalNativeBridge, String> {
    SempalNativeBridge::new(renderer, player)
}
