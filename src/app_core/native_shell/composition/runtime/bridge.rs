//! Sempal host bridge trait used by the legacy Radiant compatibility path.

use super::{
    AppModel, DirtySegments, FocusContextModel, FrameBuildResult, HotkeyResolution, KeyPress,
    NativeMotionModel, SegmentRevisions, UiAction,
};
use serde_json::Value;
use std::sync::Arc;

/// Host bridge consumed by the native runtime.
pub trait NativeAppBridge {
    /// Project the latest app model snapshot before frame build.
    ///
    /// This is the declarative render projection entrypoint:
    /// host state in, immutable view-model snapshot out.
    fn project_model(&mut self) -> Arc<AppModel>;

    /// Pull the latest app model snapshot before frame build.
    ///
    /// This compatibility shim unwraps the projected arc when callers need
    /// owned model values.
    fn pull_model(&mut self) -> AppModel {
        Arc::unwrap_or_clone(self.project_model())
    }

    /// Pull the latest app model snapshot as a shared immutable `Arc`.
    ///
    /// Runtimes can use this to avoid full-model cloning on retained cache hits
    /// when hosts already store projected models behind shared ownership.
    fn pull_model_arc(&mut self) -> Arc<AppModel> {
        self.project_model()
    }

    /// Project motion-sensitive fields only; this allows renderers to avoid
    /// full-model work on animation-only ticks.
    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        None
    }

    /// Pull motion-sensitive fields only; this allows renderers to avoid
    /// full-model work on animation-only ticks.
    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.project_motion_model()
    }

    /// Return and clear dirty projection segments produced by the latest `pull_model`.
    ///
    /// Implementations that do not track segment deltas may return
    /// [`DirtySegments::all`] to preserve conservative full-rebuild behavior.
    fn take_dirty_segments(&mut self) -> DirtySegments {
        DirtySegments::all()
    }

    /// Return static-segment revisions produced by the latest `pull_model`.
    ///
    /// Bridges that do not track segment revisions may return
    /// [`SegmentRevisions::default`] and runtimes should fall back to conservative
    /// behavior.
    fn take_segment_revisions(&mut self) -> SegmentRevisions {
        SegmentRevisions::default()
    }

    /// Resolve one keyboard gesture against the host-owned shortcut catalog.
    ///
    /// Hosts that own a command catalog should override this method and return
    /// host-defined actions through the compatibility action adapter. The
    /// default is intentionally inert so Radiant does not own application
    /// shortcut definitions.
    fn resolve_hotkey_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        _press: KeyPress,
        _focus: FocusContextModel,
    ) -> HotkeyResolution {
        HotkeyResolution::unhandled()
    }

    /// Reduce one UI action into host state.
    fn reduce_action(&mut self, _action: UiAction) {}

    /// Return whether the most recently reduced action was handled.
    ///
    /// Bridges that do not report per-action handling state may return `None`.
    /// Test harnesses should treat `Some(false)` as an explicit unhandled action
    /// signal and avoid silently counting the dispatch as covered behavior.
    fn take_last_action_handled(&mut self) -> Option<bool> {
        None
    }

    /// Install a runtime repaint signal used by background workers.
    ///
    /// Hosts that run background jobs can store this callback and forward it into
    /// worker systems so asynchronous completions can wake the UI runtime.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {}

    /// Provide the native host window handle used for external drag operations.
    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, _hwnd: isize) {}

    /// Ask the host to launch an external drag for the current active drag payload.
    ///
    /// Returns `true` when the request consumed the current runtime drag session,
    /// either because an OS drag was launched or because the host queued the
    /// selection-export path backing an external drag.
    #[cfg(target_os = "windows")]
    fn maybe_launch_external_drag(&mut self, _pointer_outside: bool, _pointer_left: bool) -> bool {
        false
    }

    /// Handle a user action emitted by runtime input processing.
    ///
    /// Compatibility shim that forwards to [`NativeAppBridge::reduce_action`].
    fn on_action(&mut self, action: UiAction) {
        self.reduce_action(action);
    }

    /// Observe one built frame result for diagnostics or telemetry.
    fn observe_frame_result(&mut self, _result: FrameBuildResult) {}

    /// Observe a built frame result for diagnostics or telemetry.
    ///
    /// Compatibility shim that forwards to
    /// [`NativeAppBridge::observe_frame_result`].
    fn on_frame_result(&mut self, result: FrameBuildResult) {
        self.observe_frame_result(result);
    }

    /// Lifecycle hook fired when the runtime is shutting down.
    fn on_runtime_exit(&mut self) -> Option<Value> {
        None
    }

    /// Lifecycle hook fired when the runtime is shutting down.
    ///
    /// Compatibility shim that forwards to
    /// [`NativeAppBridge::on_runtime_exit`].
    fn on_exit(&mut self) -> Option<Value> {
        self.on_runtime_exit()
    }
}
