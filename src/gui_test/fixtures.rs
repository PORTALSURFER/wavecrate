//! GUI fixture bridge wrappers shared by contract tests and live runtime smoke runs.

use crate::{
    app_core::{
        actions::{
            NativeAppBridge, NativeDirtySegments, NativeFrameBuildResult, NativeMotionModel,
            NativeSegmentRevisions, NativeUiAction,
        },
        gui_fixtures::build_named_gui_fixture_controller,
        ui_bridge::{WavecrateUiBridge, new_ui_bridge, new_ui_bridge_with_controller},
    },
    app_dirs::PersistenceProfileGuard,
    gui_test::{
        GuiTestModeConfig, canonical_gui_test_fixture_tag, gui_test_fixture_uses_isolated_startup,
        gui_test_fixture_uses_live_profile,
    },
    waveform::WaveformRenderer,
};
use radiant::gui::repaint::RepaintSignal;
use std::sync::Arc;
use tempfile::TempDir;

/// Controller-backed bridge wrapper that keeps fixture sandbox directories alive.
///
/// Named GUI fixtures create temporary source trees and databases. Those must
/// outlive the runtime bridge, so this wrapper owns the tempdir guards while
/// delegating all bridge behavior to `WavecrateUiBridge`.
pub struct GuiFixtureBridge {
    bridge: WavecrateUiBridge,
    _profile_guard: Option<PersistenceProfileGuard>,
    _sandbox_guards: Vec<TempDir>,
    shutdown_emitted: bool,
}

impl GuiFixtureBridge {
    /// Build one bridge for the requested fixture tag and viewport.
    ///
    /// The `live` fixture uses the normal persisted startup path. The canonical
    /// `isolated-startup` fixture exercises persisted startup against the
    /// dedicated automated-validation profile. The legacy `default` tag remains a
    /// compatibility alias for `isolated-startup`. Named controller fixtures
    /// use deterministic seeded controllers without touching user data.
    pub fn new_with_viewport(fixture_tag: &str, viewport: [u32; 2]) -> Result<Self, String> {
        if gui_test_fixture_uses_live_profile(fixture_tag) {
            let bridge = new_ui_bridge(
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
            let profile_guard = PersistenceProfileGuard::automated();
            let bridge = new_ui_bridge(
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
            bridge: new_ui_bridge_with_controller(bundle.controller),
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

    /// Project motion-only fields from the Wavecrate-owned bridge model.
    pub(crate) fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.bridge.project_motion_model()
    }

    /// Return and clear dirty segments from the Wavecrate-owned bridge model.
    pub(crate) fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        self.bridge.take_dirty_segments()
    }

    /// Return static-segment revisions from the Wavecrate-owned bridge model.
    pub(crate) fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        self.bridge.take_segment_revisions()
    }

    /// Reduce one Wavecrate-owned UI action into the fixture bridge.
    pub(crate) fn reduce_action(&mut self, action: NativeUiAction) {
        self.bridge.reduce_action(action);
    }

    /// Return whether the most recent action was handled.
    pub(crate) fn take_last_action_handled(&mut self) -> Option<bool> {
        self.bridge.take_last_action_handled()
    }

    /// Observe one Wavecrate-owned frame result.
    pub(crate) fn observe_frame_result(&mut self, result: NativeFrameBuildResult) {
        self.bridge.observe_frame_result(result);
    }

    /// Flush one bridge shutdown hook exactly once before fixture teardown.
    fn emit_runtime_exit(&mut self) -> Option<crate::native_runtime::NativeShutdownTimingArtifact> {
        if self.shutdown_emitted {
            return None;
        }
        let timing = self.bridge.on_runtime_exit();
        self.shutdown_emitted = true;
        timing
    }
}

impl NativeAppBridge for GuiFixtureBridge {
    fn project_model(&mut self) -> Arc<crate::app_core::actions::NativeAppModel> {
        <WavecrateUiBridge as NativeAppBridge>::project_model(&mut self.bridge)
    }

    fn pull_model(&mut self) -> crate::app_core::actions::NativeAppModel {
        <WavecrateUiBridge as NativeAppBridge>::pull_model(&mut self.bridge)
    }

    fn pull_model_arc(&mut self) -> Arc<crate::app_core::actions::NativeAppModel> {
        <WavecrateUiBridge as NativeAppBridge>::pull_model_arc(&mut self.bridge)
    }

    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        GuiFixtureBridge::project_motion_model(self)
    }

    fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        GuiFixtureBridge::take_dirty_segments(self)
    }

    fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        GuiFixtureBridge::take_segment_revisions(self)
    }

    fn reduce_action(&mut self, action: NativeUiAction) {
        GuiFixtureBridge::reduce_action(self, action);
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
        GuiFixtureBridge::observe_frame_result(self, result);
    }

    fn on_runtime_exit(&mut self) -> Option<crate::native_runtime::NativeShutdownTimingArtifact> {
        self.emit_runtime_exit()
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
    #[ignore = "runs through scripts/gui.ps1 contract; fixture-backed smoke is too expensive for the default lib test lane"]
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
    #[ignore = "runs through scripts/gui.ps1 contract; fixture-backed smoke is too expensive for the default lib test lane"]
    fn named_fixtures_stay_controller_backed() {
        let bridge = GuiFixtureBridge::new("browser").expect("browser fixture bridge");
        assert!(bridge._profile_guard.is_none());
        assert!(!bridge._sandbox_guards.is_empty());
    }
}
