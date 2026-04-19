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
    app_dirs::PersistenceProfileGuard,
    gui::repaint::RepaintSignal,
    gui_test::{
        GuiTestModeConfig, canonical_gui_test_fixture_tag, gui_test_fixture_uses_isolated_startup,
        gui_test_fixture_uses_live_profile,
    },
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
    _profile_guard: Option<PersistenceProfileGuard>,
    _sandbox_guards: Vec<TempDir>,
    shutdown_emitted: bool,
}

impl GuiFixtureBridge {
    /// Build one bridge for the requested fixture tag and viewport.
    ///
    /// The `live` fixture uses the normal persisted startup path. The canonical
    /// `isolated-startup` fixture exercises persisted startup against a
    /// dedicated non-live profile. The legacy `default` tag remains a
    /// compatibility alias for `isolated-startup`. Named controller fixtures
    /// use deterministic seeded controllers without touching user data.
    pub fn new_with_viewport(fixture_tag: &str, viewport: [u32; 2]) -> Result<Self, String> {
        if gui_test_fixture_uses_live_profile(fixture_tag) {
            let bridge = new_native_bridge(
                WaveformRenderer::new(viewport[0].max(320), viewport[1].max(180)),
                None,
            )?;
            return Ok(Self {
                bridge,
                _profile_guard: None,
                _sandbox_guards: Vec::new(),
                shutdown_emitted: false,
            });
        }
        if gui_test_fixture_uses_isolated_startup(fixture_tag) {
            let profile_guard = PersistenceProfileGuard::named("gui-test");
            let bridge = new_native_bridge(
                WaveformRenderer::new(viewport[0].max(320), viewport[1].max(180)),
                None,
            )?;
            return Ok(Self {
                bridge,
                _profile_guard: Some(profile_guard),
                _sandbox_guards: Vec::new(),
                shutdown_emitted: false,
            });
        }
        let bundle = build_named_gui_fixture_controller(
            WaveformRenderer::new(viewport[0].max(320), viewport[1].max(180)),
            canonical_gui_test_fixture_tag(fixture_tag),
        )?;
        Ok(Self {
            bridge: new_native_bridge_with_controller(bundle.controller),
            _profile_guard: None,
            _sandbox_guards: bundle.sandbox_guards,
            shutdown_emitted: false,
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

    /// Flush one bridge shutdown hook exactly once before fixture teardown.
    fn emit_runtime_exit(&mut self) {
        if self.shutdown_emitted {
            return;
        }
        self.bridge.on_runtime_exit();
        self.shutdown_emitted = true;
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

    fn take_last_action_handled(&mut self) -> Option<bool> {
        self.bridge.take_last_action_handled()
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.bridge.install_repaint_signal(signal);
    }

    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, hwnd: isize) {
        self.bridge.set_external_drag_hwnd(hwnd);
    }

    #[cfg(target_os = "windows")]
    fn maybe_launch_external_drag(&mut self, pointer_outside: bool, pointer_left: bool) -> bool {
        self.bridge
            .maybe_launch_external_drag(pointer_outside, pointer_left)
    }

    fn observe_frame_result(&mut self, result: NativeFrameBuildResult) {
        self.bridge.observe_frame_result(result);
    }

    fn on_runtime_exit(&mut self) {
        self.emit_runtime_exit();
    }
}

impl Drop for GuiFixtureBridge {
    fn drop(&mut self) {
        self.emit_runtime_exit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_test::GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG;

    #[test]
    fn default_fixture_alias_matches_isolated_startup_behavior() {
        let default_bridge = GuiFixtureBridge::new("default").expect("default fixture bridge");
        let isolated_bridge = GuiFixtureBridge::new(GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG)
            .expect("isolated startup bridge");
        assert_eq!(
            default_bridge._profile_guard.is_some(),
            isolated_bridge._profile_guard.is_some()
        );
        assert!(default_bridge._sandbox_guards.is_empty());
        assert!(isolated_bridge._sandbox_guards.is_empty());
    }

    #[test]
    fn named_fixtures_stay_controller_backed() {
        let bridge = GuiFixtureBridge::new("browser").expect("browser fixture bridge");
        assert!(bridge._profile_guard.is_none());
        assert!(!bridge._sandbox_guards.is_empty());
    }
}
