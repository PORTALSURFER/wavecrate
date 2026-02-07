//! Shared GUI runtime host implementations re-exported from `radiant`.

/// RGBA icon payload used by native runtime hosts.
pub type WindowIconRgba = radiant::gui_runtime::WindowIconRgba;
/// Native runtime launch options for vello hosts.
pub type NativeRunOptions = radiant::gui_runtime::EguiRunOptions;

pub use radiant::gui_runtime::{run_native_vello_app, run_native_vello_preview};
