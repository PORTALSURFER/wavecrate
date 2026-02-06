//! Shared GUI runtime host implementations re-exported from `radiant`.

pub use radiant::gui_runtime::{
    EguiAppRuntime, EguiRunOptions, WindowIconRgba, run_egui_wgpu_app, run_native_vello_app,
    run_native_vello_preview,
};
