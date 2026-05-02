//! Sempal native Vello compatibility facade used by the legacy Radiant path.

use super::{AppModel, GuiAutomationSnapshot, NativeAppBridge, NativeRunReport};
use crate::gui::{
    native_shell::{NativeShellState, ShellLayout, ShellLayoutRuntime, StyleTokens},
    types::Vector2,
};
use crate::gui_runtime::{NativeRunOptions, native_vello};

/// Run the native Vello backend window with a host-provided legacy shell bridge.
///
/// The runtime loop is owned by winit and blocks until the native window
/// closes. The host receives user input each frame through the bridge-driven
/// action path, and this function returns the host result from the event loop
/// invocation.
pub fn run_native_vello_app_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    native_vello::run_legacy_shell_vello_app_with_artifacts(options, bridge)
}

/// Run the native Vello backend window with a host-provided legacy shell bridge.
pub fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    run_native_vello_app_with_artifacts(options, bridge).result
}

/// Capture a deterministic native-shell automation snapshot without launching a window.
pub fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &AppModel,
) -> GuiAutomationSnapshot {
    let viewport = Vector2::new(viewport[0].max(1.0), viewport[1].max(1.0));
    let style = StyleTokens::for_viewport_width(viewport.x);
    let mut runtime = ShellLayoutRuntime::default();
    let layout = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
    let mut shell_state = NativeShellState::new();
    shell_state.sync_from_model(model);
    shell_state.automation_snapshot(&layout, model)
}
