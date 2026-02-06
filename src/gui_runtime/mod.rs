//! Shared GUI runtime host implementations.

mod egui_wgpu;

pub use egui_wgpu::{
    EguiAppRuntime, EguiRunOptions, WindowIconRgba, run_egui_wgpu_app,
};
