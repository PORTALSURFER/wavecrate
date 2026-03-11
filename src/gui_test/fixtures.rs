//! GUI fixture bridge wrappers shared by contract tests and live runtime smoke runs.

use crate::{
    app_core::{
        actions::{
            NativeAppBridge, NativeDirtySegments, NativeFrameBuildResult, NativeMotionModel,
            NativeSegmentRevisions, NativeUiAction,
        },
        controller::build_named_gui_fixture_controller,
        native_bridge::{SempalNativeBridge, new_native_bridge, new_native_bridge_with_controller},
    },
    gui::repaint::RepaintSignal,
    gui_test::GuiTestModeConfig,
    waveform::WaveformRenderer,
};
use std::sync::Arc;
use tempfile::TempDir;

/// Controller-backed bridge wrapper that keeps fixture sandbox directories alive.
///
/// Named GUI fixtures create temporary source trees and databases. Those must
/// outlive the runtime bridge, so this wrapper owns the tempdir guards while
/// delegating all bridge behavior to `SempalNativeBridge`.
pub struct GuiFixtureBridge {
    bridge: SempalNativeBridge,
    _sandbox_guards: Vec<TempDir>,
}

impl GuiFixtureBridge {
    /// Build one bridge for the requested fixture tag and viewport.
    ///
    /// The `default` fixture uses the normal persisted startup path. Named
    /// fixtures use deterministic seeded controllers without touching user data.
    pub fn new_with_viewport(fixture_tag: &str, viewport: [u32; 2]) -> Result<Self, String> {
        if fixture_tag == "default" {
            let bridge = new_native_bridge(
                WaveformRenderer::new(viewport[0].max(320), viewport[1].max(180)),
                None,
            )?;
            return Ok(Self {
                bridge,
                _sandbox_guards: Vec::new(),
            });
        }
        let bundle = build_named_gui_fixture_controller(
            WaveformRenderer::new(viewport[0].max(320), viewport[1].max(180)),
            fixture_tag,
        )?;
        Ok(Self {
            bridge: new_native_bridge_with_controller(bundle.controller),
            _sandbox_guards: bundle.sandbox_guards,
        })
    }

    /// Build one bridge using the default deterministic GUI viewport.
    pub fn new(fixture_tag: &str) -> Result<Self, String> {
        Self::new_with_viewport(fixture_tag, GuiTestModeConfig::default().viewport)
    }

    /// Enable live GUI test artifact emission for this bridge instance.
    pub fn install_gui_test_mode(&mut self, config: GuiTestModeConfig) {
        self.bridge.install_gui_test_mode(config);
    }
}

impl NativeAppBridge for GuiFixtureBridge {
    fn project_model(&mut self) -> Arc<radiant::app::AppModel> {
        self.bridge.project_model()
    }

    fn pull_model(&mut self) -> radiant::app::AppModel {
        self.bridge.pull_model()
    }

    fn pull_model_arc(&mut self) -> Arc<radiant::app::AppModel> {
        self.bridge.pull_model_arc()
    }

    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.bridge.project_motion_model()
    }

    fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        self.bridge.take_dirty_segments()
    }

    fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        self.bridge.take_segment_revisions()
    }

    fn reduce_action(&mut self, action: NativeUiAction) {
        self.bridge.reduce_action(action);
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.bridge.install_repaint_signal(signal);
    }

    fn observe_frame_result(&mut self, result: NativeFrameBuildResult) {
        self.bridge.observe_frame_result(result);
    }

    fn on_runtime_exit(&mut self) {
        self.bridge.on_runtime_exit();
    }
}
