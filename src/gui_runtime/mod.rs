//! Shared GUI runtime host implementations.

mod egui_wgpu;
mod native_vello;

pub use egui_wgpu::{
    EguiAppRuntime, EguiRunOptions, WindowIconRgba, run_egui_wgpu_app,
};
pub use native_vello::run_native_vello_preview;
