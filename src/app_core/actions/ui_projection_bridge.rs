//! Wavecrate-owned UI runtime bridge trait.
//!
//! Wavecrate code implements this trait so projected DTOs and actions remain owned
//! in `app_core` while the UI runtime adapter handles the Radiant launch
//! boundary.

use super::{
    NativeAppModel, NativeDirtySegments, NativeFrameBuildResult, NativeMotionModel,
    NativeSegmentRevisions, NativeUiAction,
};
use crate::native_runtime::NativeShutdownTimingArtifact;
use radiant::gui::repaint::RepaintSignal;
use std::{path::PathBuf, sync::Arc};

/// Host bridge used by Wavecrate's UI runtime adapter.
pub trait NativeAppBridge {
    /// Project the latest app model snapshot before frame build.
    fn project_model(&mut self) -> Arc<NativeAppModel>;

    /// Pull the latest app model snapshot before frame build.
    fn pull_model(&mut self) -> NativeAppModel {
        Arc::unwrap_or_clone(self.project_model())
    }

    /// Pull the latest app model snapshot as a shared immutable `Arc`.
    fn pull_model_arc(&mut self) -> Arc<NativeAppModel> {
        self.project_model()
    }

    /// Project motion-sensitive fields only.
    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        None
    }

    /// Pull motion-sensitive fields only.
    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.project_motion_model()
    }

    /// Return and clear dirty projection segments produced by the latest model pull.
    fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        NativeDirtySegments::all()
    }

    /// Return static-segment revisions produced by the latest model pull.
    fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        NativeSegmentRevisions::default()
    }

    /// Reduce one UI action into host state.
    fn reduce_action(&mut self, _action: NativeUiAction) {}

    /// Handle one native file drag/drop event delivered by the Radiant runtime.
    fn handle_native_file_drop(&mut self, _event: NativeFileDropEvent) {}

    /// Return whether the most recently reduced action was handled.
    fn take_last_action_handled(&mut self) -> Option<bool> {
        None
    }

    /// Install a runtime repaint signal used by background workers.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {}

    /// Provide the native host window handle used for external drag operations.
    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, _hwnd: isize) {}

    /// Ask the host to launch an external drag for the current active drag payload.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn maybe_launch_external_drag(&mut self, _pointer_outside: bool, _pointer_left: bool) -> bool {
        false
    }

    /// Handle a user action emitted by runtime input processing.
    fn on_action(&mut self, action: NativeUiAction) {
        self.reduce_action(action);
    }

    /// Observe one built frame result for diagnostics or telemetry.
    fn observe_frame_result(&mut self, _result: NativeFrameBuildResult) {}

    /// Observe a built frame result for diagnostics or telemetry.
    fn on_frame_result(&mut self, result: NativeFrameBuildResult) {
        self.observe_frame_result(result);
    }

    /// Lifecycle hook fired when the runtime is shutting down.
    fn on_runtime_exit(&mut self) -> Option<NativeShutdownTimingArtifact> {
        None
    }

    /// Lifecycle hook fired when the runtime is shutting down.
    fn on_exit(&mut self) -> Option<NativeShutdownTimingArtifact> {
        self.on_runtime_exit()
    }
}

/// Native file drag/drop phase delivered through Wavecrate's retained shell bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeFileDropPhase {
    /// A native file drag is hovering over the application window.
    Hover,
    /// A native file drag left the application window or was cancelled.
    Cancel,
    /// A native file was dropped onto the application window.
    Drop,
}

/// Native file drag/drop event delivered through Wavecrate's retained shell bridge.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeFileDropEvent {
    /// Event phase.
    pub phase: NativeFileDropPhase,
    /// File path supplied by the operating system, when present.
    pub path: Option<PathBuf>,
    /// Last known logical pointer position in the surface.
    pub position: Option<(f32, f32)>,
}
