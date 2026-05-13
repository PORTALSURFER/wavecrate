#![deny(missing_docs)]
#![deny(warnings)]
#![allow(clippy::too_many_arguments)]

//! Feature-selected Wavecrate application entry point.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[cfg(all(feature = "legacy-gui", feature = "radiant-gui"))]
compile_error!("enable either legacy-gui or radiant-gui, not both");

#[cfg(not(any(feature = "legacy-gui", feature = "radiant-gui")))]
compile_error!("enable either legacy-gui or radiant-gui");

#[cfg(feature = "legacy-gui")]
mod app_icon;
#[cfg(feature = "radiant-gui")]
mod gui_app;
#[cfg(feature = "legacy-gui")]
mod legacy_gui_app;
#[cfg(feature = "legacy-gui")]
mod run_contract;

#[cfg(feature = "radiant-gui")]
fn main() -> Result<(), String> {
    gui_app::run()
}

#[cfg(feature = "legacy-gui")]
fn main() -> Result<(), String> {
    legacy_gui_app::run()
}
