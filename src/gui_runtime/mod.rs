//! Shared GUI runtime host implementations re-exported from `radiant`.
//!
//! The runtime layer in `sempal` is intentionally thin: this module only
//! translates and forwards runtime launch options to `radiant` and mirrors host-
//! level errors to tracing for diagnostics.
//! No widget state, layout rules, rendering commands, or input policies are
//! implemented here.

use crate::app_core::actions::NativeAppBridge;
use tracing::{error, info};

/// RGBA icon payload used by native runtime hosts.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}
/// Native runtime launch options for Vello hosts.
#[derive(Clone, Debug, Default)]
pub struct NativeRunOptions {
    /// Window title.
    pub title: String,
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
    /// Whether the window starts maximized.
    pub maximized: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
    /// Target frame rate for animation-driven redraws.
    pub target_fps: u32,
}

impl From<NativeRunOptions> for radiant::gui_runtime::NativeRunOptions {
    fn from(value: NativeRunOptions) -> Self {
        Self {
            title: value.title,
            inner_size: value.inner_size,
            min_inner_size: value.min_inner_size,
            maximized: value.maximized,
            icon: value.icon.map(Into::into),
            target_fps: value.target_fps,
        }
    }
}

impl From<WindowIconRgba> for radiant::gui_runtime::WindowIconRgba {
    fn from(value: WindowIconRgba) -> Self {
        Self {
            rgba: value.rgba,
            width: value.width,
            height: value.height,
        }
    }
}

/// Run the native Vello backend with a host-provided application bridge.
///
/// This call blocks until the native host exits and returns an error if startup
/// or runtime execution fails.
pub fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    info!("Launching radiant native Vello runtime");
    let result =
        radiant::gui_runtime::run_native_vello_app(options.into(), bridge).map_err(|err| {
            error!(%err, "radiant native Vello runtime returned error");
            err
        });

    if result.is_ok() {
        info!("Radiant native Vello runtime returned successfully");
    }

    result
}

/// Run the native Vello backend preview shell for backend smoke testing.
///
/// This is typically used to validate host integration behavior without passing a
/// full application bridge.
pub fn run_native_vello_preview(options: NativeRunOptions) -> Result<(), String> {
    info!("Launching radiant native Vello preview runtime");
    let result = radiant::gui_runtime::run_native_vello_preview(options.into()).map_err(|err| {
        error!(%err, "radiant native Vello preview returned error");
        err
    });

    if result.is_ok() {
        info!("Radiant native Vello preview returned successfully");
    }

    result
}
